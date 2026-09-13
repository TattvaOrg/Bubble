use qmetaobject::*;
use std::process::Command;

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct RemoteAccessService {
    _base: qt_base_class!(trait QObject),

    pub busy: qt_property!(bool; NOTIFY busyChanged),
    pub busyChanged: qt_signal!(),
    pub connectionFinished: qt_signal!(success: bool, uri: QString, error: QString),

    pub buildUri: qt_method!(fn(&self, protocol: QString, host: QString, remote_path: QString, user: QString, port: i32, share: QString) -> QString),
    pub connectToLocation: qt_method!(fn(&mut self, uri: QString)),
}

impl RemoteAccessService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn buildUri(
        &self,
        protocol: QString,
        host: QString,
        remote_path: QString,
        user: QString,
        port: i32,
        share: QString,
    ) -> QString {
        let host_str = host.to_string().trim().to_string();
        if host_str.is_empty() {
            return QString::default();
        }
        let proto_str = protocol.to_string().trim().to_lowercase();
        let user_str = user.to_string().trim().to_string();
        let share_str = share.to_string().trim().to_string();
        let path_str = remote_path.to_string().trim().to_string();

        let full_path = if proto_str == "smb" {
            if !share_str.is_empty() {
                if path_str.is_empty() {
                    format!("/{}", share_str)
                } else if path_str.starts_with('/') {
                    format!("/{}{}", share_str, path_str)
                } else {
                    format!("/{}/{}", share_str, path_str)
                }
            } else if path_str.starts_with('/') {
                path_str
            } else if path_str.is_empty() {
                "/".to_string()
            } else {
                format!("/{}", path_str)
            }
        } else if path_str.starts_with('/') {
            path_str
        } else if path_str.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", path_str)
        };

        let mut uri = format!("{}://", proto_str);
        if !user_str.is_empty() {
            uri.push_str(&user_str);
            uri.push('@');
        }
        uri.push_str(&host_str);
        if port > 0 {
            uri.push_str(&format!(":{}", port));
        }
        uri.push_str(&full_path);
        QString::from(uri.as_str())
    }

    pub fn connectToLocation(&mut self, uri: QString) {
        let uri_str = uri.to_string().trim().to_string();
        if uri_str.is_empty() {
            self.connectionFinished(false, QString::default(), QString::from("Enter a remote location"));
            return;
        }

        self.busy = true;
        self.busyChanged();

        let output = Command::new("gio")
            .arg("mount")
            .arg(&uri_str)
            .output();

        self.busy = false;
        self.busyChanged();

        match output {
            Ok(out) => {
                let success = out.status.success();
                let err_str = String::from_utf8_lossy(&out.stderr).trim().to_string();
                self.connectionFinished(success, QString::from(uri_str.as_str()), QString::from(err_str.as_str()));
            }
            Err(e) => {
                self.connectionFinished(false, QString::from(uri_str.as_str()), QString::from(e.to_string().as_str()));
            }
        }
    }
}
