use std::env;
use std::ffi::CString;
use std::process;

const FS_IMMUTABLE_FL: libc::c_long = 0x00000010;
const FS_IOC_GETFLAGS: libc::c_ulong = 0x80086601;
const FS_IOC_SETFLAGS: libc::c_ulong = 0x40086602;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: bubble-vault-helper [+i|-i] <path>");
        process::exit(1);
    }

    let flag = &args[1];
    let path = &args[2];

    if flag != "+i" && flag != "-i" {
        eprintln!("Invalid flag. Use +i or -i");
        process::exit(1);
    }

    let c_path = match CString::new(path.as_str()) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Invalid path string");
            process::exit(1);
        }
    };

    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::lstat(c_path.as_ptr(), &mut st) } != 0 {
        eprintln!("Error: path does not exist: {}", std::io::Error::last_os_error());
        process::exit(1);
    }

    // Refuse symlinks
    if (st.st_mode & libc::S_IFMT) == libc::S_IFLNK {
        eprintln!("Error: refusing to operate on symlink");
        process::exit(1);
    }

    // Security check: caller UID must own the file (unless caller is root)
    let caller_uid = unsafe { libc::getuid() };
    if caller_uid != 0 && st.st_uid != caller_uid {
        eprintln!(
            "Error: caller UID {} does not own {} (owned by UID {})",
            caller_uid, path, st.st_uid
        );
        process::exit(1);
    }

    // Elevate to root effective UID if running with setuid root
    unsafe {
        let _ = libc::seteuid(0);
    }

    let is_dir = (st.st_mode & libc::S_IFMT) == libc::S_IFDIR;
    let mut open_flags = libc::O_RDONLY | libc::O_NONBLOCK;
    if is_dir {
        open_flags |= libc::O_DIRECTORY;
    }

    let fd = unsafe { libc::open(c_path.as_ptr(), open_flags) };
    if fd < 0 {
        eprintln!("Error opening path: {}", std::io::Error::last_os_error());
        process::exit(1);
    }

    struct FdGuard(libc::c_int);
    impl Drop for FdGuard {
        fn drop(&mut self) {
            unsafe { libc::close(self.0) };
        }
    }
    let _guard = FdGuard(fd);

    let mut flags: libc::c_long = 0;
    if unsafe { libc::ioctl(fd, FS_IOC_GETFLAGS, &mut flags) } < 0 {
        eprintln!("ioctl(FS_IOC_GETFLAGS) failed: {}", std::io::Error::last_os_error());
        process::exit(1);
    }

    if flag == "+i" {
        flags |= FS_IMMUTABLE_FL;
    } else {
        flags &= !FS_IMMUTABLE_FL;
    }

    if unsafe { libc::ioctl(fd, FS_IOC_SETFLAGS, &flags) } < 0 {
        eprintln!("ioctl(FS_IOC_SETFLAGS) failed: {}", std::io::Error::last_os_error());
        process::exit(1);
    }
}
