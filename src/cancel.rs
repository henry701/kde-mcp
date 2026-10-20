//! Global cancel shortcut — Meta+Esc via KGlobalAccel, with SIGUSR1 fallback.
//!
//! The shortcut is registered with `org.kde.kglobalaccel` so KWin grabs the
//! key at compositor level (prompt-injection-safe — apps never see it).
//! When the user presses Meta+Esc, kglobalaccel fires
//! `globalShortcutPressed` on the component path returned by KDE; we listen and
//! flip the `CancellationToken` shared with the policy gate.
//!
//! Registration is best-effort. If kglobalaccel rejects (already-registered
//! action, daemon not running, etc.), SIGUSR1 still works as fallback:
//! `pkill -USR1 kde-mcp`. The tray icon's "Abort" menu item is the
//! third route.

use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;
use zbus::Connection;

// Qt key + modifier constants. Combined Qt::Key value for "Meta+Escape".
const QT_KEY_ESCAPE: i32 = 0x01000000;
const QT_MOD_META: i32 = 0x10000000;
const META_ESCAPE: i32 = QT_KEY_ESCAPE | QT_MOD_META;

const COMPONENT_UNIQUE: &str = "kde-mcp";
const COMPONENT_FRIENDLY: &str = "kde-mcp (desktop control)";
const ACTION_UNIQUE: &str = "cancel";
const ACTION_FRIENDLY: &str = "Abort current operation";

#[zbus::proxy(
    interface = "org.kde.KGlobalAccel",
    default_service = "org.kde.kglobalaccel",
    default_path = "/kglobalaccel"
)]
trait KGlobalAccel {
    #[zbus(name = "doRegister")]
    fn do_register(&self, action_id: Vec<&str>) -> zbus::Result<()>;

    #[zbus(name = "setShortcut")]
    fn set_shortcut(
        &self,
        action_id: Vec<&str>,
        keys: Vec<i32>,
        flags: u32,
    ) -> zbus::Result<Vec<i32>>;

    #[zbus(name = "getComponent")]
    fn get_component(
        &self,
        component_unique: &str,
    ) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
}

#[zbus::proxy(
    interface = "org.kde.kglobalaccel.Component",
    default_service = "org.kde.kglobalaccel"
)]
trait Component {
    #[zbus(signal, name = "globalShortcutPressed")]
    fn global_shortcut_pressed(
        &self,
        component_unique: String,
        shortcut_unique: String,
        timestamp: i64,
    ) -> zbus::Result<()>;
}

/// Install the cancel token. Spawns:
///  - SIGUSR1 handler (always works)
///  - KGlobalAccel Meta+Esc registration + signal listener (best-effort)
pub async fn install(bus: &Connection) -> CancellationToken {
    let token = CancellationToken::new();
    spawn_sigusr1(token.clone());
    if let Err(e) = register_kglobalaccel(bus, token.clone()).await {
        tracing::warn!(error=%e, "kglobalaccel registration failed — Meta+Esc inactive; use SIGUSR1 or tray Abort instead");
    } else {
        tracing::info!("Meta+Esc registered with KGlobalAccel");
    }
    token
}

fn spawn_sigusr1(token: CancellationToken) {
    tokio::spawn(async move {
        let mut sig =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::user_defined1()) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(error=%e, "SIGUSR1 handler install failed");
                    return;
                }
            };
        sig.recv().await;
        tracing::info!("cancel: SIGUSR1 received");
        token.cancel();
        crate::notify::notify_cancelled().await;
    });
}

async fn register_kglobalaccel(
    bus: &Connection,
    token: CancellationToken,
) -> Result<(), zbus::Error> {
    let kga = KGlobalAccelProxy::new(bus).await?;
    let action_id = vec![
        COMPONENT_UNIQUE,
        ACTION_UNIQUE,
        COMPONENT_FRIENDLY,
        ACTION_FRIENDLY,
    ];
    // doRegister is idempotent for an already-registered component — it
    // resurrects the action. setShortcut then (re)binds it.
    kga.do_register(action_id.clone()).await?;
    // flags=0x4 = NoAutoloading (don't read existing config); 0x2 = SetPresent
    // (mark as the current shortcut). KGlobalAccelD::SetShortcutFlag enum in
    // KDE source. Using NoAutoloading|SetPresent so we set Meta+Esc even if
    // user previously rebound something.
    let granted = kga.set_shortcut(action_id, vec![META_ESCAPE], 0x6).await?;
    if !granted.contains(&META_ESCAPE) {
        return Err(zbus::Error::Failure(
            "KGlobalAccel did not grant Meta+Esc".into(),
        ));
    }

    // Component IDs may contain characters that are invalid in D-Bus paths.
    let component_path = kga.get_component(COMPONENT_UNIQUE).await?;
    let component = ComponentProxy::builder(bus)
        .path(component_path)?
        .build()
        .await?;
    let mut stream = component.receive_global_shortcut_pressed().await?;
    tokio::spawn(async move {
        if let Some(_msg) = stream.next().await {
            tracing::info!("cancel: Meta+Esc pressed via KGlobalAccel");
            token.cancel();
            crate::notify::notify_cancelled().await;
        }
        // Token never un-cancels; once flipped, subsequent presses are no-
        // ops. The server stays "shut down for input" until restart —
        // intentional: a cancelled session implies the operator wants to
        // stop, full stop.
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::process::{Child, Command, Stdio};
    use zbus::object_server::SignalEmitter;

    struct PrivateBus(Child);

    impl Drop for PrivateBus {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    struct Accelerator;

    #[zbus::interface(name = "org.kde.KGlobalAccel")]
    impl Accelerator {
        #[zbus(name = "doRegister")]
        fn do_register(&self, action_id: Vec<String>) -> zbus::fdo::Result<()> {
            if action_id
                != [
                    COMPONENT_UNIQUE,
                    ACTION_UNIQUE,
                    COMPONENT_FRIENDLY,
                    ACTION_FRIENDLY,
                ]
            {
                return Err(zbus::fdo::Error::InvalidArgs(
                    "incorrect action ID order".into(),
                ));
            }
            Ok(())
        }

        #[zbus(name = "setShortcut")]
        fn set_shortcut(&self, _action_id: Vec<String>, keys: Vec<i32>, _flags: u32) -> Vec<i32> {
            keys
        }

        #[zbus(name = "getComponent")]
        fn get_component(&self, _component_unique: String) -> zbus::zvariant::OwnedObjectPath {
            "/component/kde_mcp".try_into().unwrap()
        }
    }

    struct Shortcut;

    #[zbus::interface(name = "org.kde.kglobalaccel.Component")]
    impl Shortcut {
        #[zbus(signal, name = "globalShortcutPressed")]
        async fn global_shortcut_pressed(
            emitter: &SignalEmitter<'_>,
            component_unique: &str,
            shortcut_unique: &str,
            timestamp: i64,
        ) -> zbus::Result<()>;
    }

    #[tokio::test]
    async fn registration_uses_kde_action_order_and_resolved_component_path() {
        let mut daemon = Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut address = String::new();
        BufReader::new(daemon.stdout.take().unwrap())
            .read_line(&mut address)
            .unwrap();
        let _daemon = PrivateBus(daemon);
        let server = zbus::connection::Builder::address(address.trim())
            .unwrap()
            .name("org.kde.kglobalaccel")
            .unwrap()
            .serve_at("/kglobalaccel", Accelerator)
            .unwrap()
            .serve_at("/component/kde_mcp", Shortcut)
            .unwrap()
            .build()
            .await
            .unwrap();
        let client = zbus::connection::Builder::address(address.trim())
            .unwrap()
            .build()
            .await
            .unwrap();
        let token = CancellationToken::new();
        register_kglobalaccel(&client, token.clone()).await.unwrap();
        let emitter = SignalEmitter::new(&server, "/component/kde_mcp").unwrap();
        Shortcut::global_shortcut_pressed(&emitter, COMPONENT_UNIQUE, ACTION_UNIQUE, 1)
            .await
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), token.cancelled())
            .await
            .unwrap();
    }
}
