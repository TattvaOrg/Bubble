// Handed to 7z/unzip whenever no password is known. It is deliberately wrong,
// and long enough that no real password collides with it: without a -p/-P the
// tools prompt on stdin and wait forever for input nobody will type, hanging
// the extraction. With it, an encrypted archive fails immediately and an
// unencrypted one ignores it.
pub const ARCHIVE_PASSWORD_SENTINEL: &str = "__bubble_placeholder_password__";

pub fn effective_archive_password(password: &str) -> String {
    if password.is_empty() {
        ARCHIVE_PASSWORD_SENTINEL.to_string()
    } else {
        password.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effective_archive_password() {
        assert_eq!(effective_archive_password(""), ARCHIVE_PASSWORD_SENTINEL);
        assert_eq!(effective_archive_password("secret"), "secret");
    }
}
