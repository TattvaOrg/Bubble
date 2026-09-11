#include <iostream>
#include <string>
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <linux/fs.h>
#include <sys/stat.h>
#include <cstring>
#include <cerrno>

int main(int argc, char *argv[])
{
    if (argc != 3) {
        std::cerr << "Usage: bubble-vault-helper [+i|-i] <path>\n";
        return 1;
    }

    std::string flag = argv[1];
    const char *path = argv[2];

    if (flag != "+i" && flag != "-i") {
        std::cerr << "Invalid flag. Use +i or -i\n";
        return 1;
    }

    struct stat st;
    if (lstat(path, &st) != 0) {
        std::cerr << "Error: path does not exist: " << strerror(errno) << "\n";
        return 1;
    }

    // Do not set immutable flag on symlinks
    if (S_ISLNK(st.st_mode)) {
        std::cerr << "Error: refusing to operate on symlink\n";
        return 1;
    }

    // Security check: caller must own the file/dir (unless caller is already root)
    uid_t callerUid = getuid();
    if (callerUid != 0 && st.st_uid != callerUid) {
        std::cerr << "Error: caller UID " << callerUid << " does not own " << path
                  << " (owned by UID " << st.st_uid << ")\n";
        return 1;
    }

    // Elevate to root effective UID if running with setuid root
    if (seteuid(0) != 0) {
        // Non-fatal if not setuid root
    }

    int openFlags = O_RDONLY | O_NONBLOCK;
    if (S_ISDIR(st.st_mode)) {
        openFlags |= O_DIRECTORY;
    }

    int fd = open(path, openFlags);
    if (fd < 0) {
        std::cerr << "Error opening path: " << strerror(errno) << "\n";
        return 1;
    }

    long flags = 0;
    if (ioctl(fd, FS_IOC_GETFLAGS, &flags) < 0) {
        std::cerr << "ioctl(FS_IOC_GETFLAGS) failed: " << strerror(errno) << "\n";
        close(fd);
        return 1;
    }

    if (flag == "+i") {
        flags |= FS_IMMUTABLE_FL;
    } else {
        flags &= ~FS_IMMUTABLE_FL;
    }

    if (ioctl(fd, FS_IOC_SETFLAGS, &flags) < 0) {
        std::cerr << "ioctl(FS_IOC_SETFLAGS) failed: " << strerror(errno) << "\n";
        close(fd);
        return 1;
    }

    close(fd);
    return 0;
}
