use qmetaobject::*;
use std::process::Command;

#[allow(non_snake_case)]
#[derive(QObject, Default)]
pub struct RuntimeFeaturesService {
    _base: qt_base_class!(trait QObject),

    pub useIntegratedWindowControls: qt_property!(bool; NOTIFY dummy),
    pub udisksctlAvailable: qt_property!(bool; NOTIFY dummy),
    pub ffmpegAvailable: qt_property!(bool; NOTIFY dummy),
    pub batAvailable: qt_property!(bool; NOTIFY dummy),

    pub dummy: qt_signal!(),

    pub installHint: qt_method!(fn(&self, feature: QString) -> QString),
}

impl RuntimeFeaturesService {
    pub fn new() -> Self {
        let has_cmd = |cmd: &str| Command::new("which").arg(cmd).output().map(|o| o.status.success()).unwrap_or(false);

        Self {
            useIntegratedWindowControls: true,
            udisksctlAvailable: has_cmd("udisksctl"),
            ffmpegAvailable: has_cmd("ffmpeg"),
            batAvailable: has_cmd("bat") || has_cmd("batcat"),
            ..Default::default()
        }
    }

    pub fn installHint(&self, feature: QString) -> QString {
        let f = feature.to_string();
        let hint = match f.as_str() {
            "deviceMount" => "Install 'udisks2' for drive mounting and unmounting support.",
            "textHighlight" => "Install 'bat' for syntax-highlighted text file previews.",
            "pdfPreview" => "Install 'pdftoppm' (poppler-utils) for PDF thumbnailing and previews.",
            "videoPreview" => "Install 'ffmpeg' for video thumbnail generation.",
            "remoteAccess" => "Install 'gvfs' or 'kio' for remote network filesystem mounting.",
            "smbRemoteAccess" => "Install 'gvfs-backends' or 'samba-client' for Windows SMB shares.",
            _ => "Package required for this feature.",
        };
        QString::from(hint)
    }
}
