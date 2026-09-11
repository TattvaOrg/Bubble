#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QProcess>
#include <QDebug>
#include <QThread>
#include <iostream>
#include <csignal>
#include <sys/xattr.h>
#include "services/cryptoengine.h"
#include "services/vaultdatabase.h"

static void unlockChattr(const QString &path)
{
    QStringList args;
    args << "-i" << path;
    QProcess::execute("chattr", args);
}

static void shredAndRemove(const QString &path)
{
    QFileInfo info(path);
    if (!info.exists()) {
        return;
    }

    unlockChattr(path);

    if (info.isDir()) {
        QDir dir(path);
        const auto entries = dir.entryInfoList(QDir::Files | QDir::Dirs | QDir::NoDotAndDotDot | QDir::Hidden);
        for (const auto &entry : entries) {
            shredAndRemove(entry.absoluteFilePath());
        }
        dir.rmdir(path);
    } else {
        std::cout << "Shredding locked file: " << path.toStdString() << std::endl;
        CryptoEngine::shredFile(path);
    }
}

static void processVaultDb(const QString &dbPath)
{
    if (!QFile::exists(dbPath)) {
        return;
    }

    std::cout << "Processing vault at: " << dbPath.toStdString() << std::endl;
    VaultDatabase db;
    if (!db.open(dbPath)) {
        std::cerr << "Failed to open database: " << dbPath.toStdString() << std::endl;
        return;
    }

    const QStringList paths = db.allLockedPaths();
    for (const QString &path : paths) {
        shredAndRemove(path);
    }

    db.close();

    // Remove the database files
    QFile::remove(dbPath);
    QFile::remove(dbPath + "-wal");
    QFile::remove(dbPath + "-shm");
    std::cout << "Vault database destroyed: " << dbPath.toStdString() << std::endl;
}

int main(int argc, char *argv[])
{
    QCoreApplication app(argc, argv);
    const QStringList args = app.arguments();

    // Mode: Watch an open file's PID and re-encrypt + lock when the PID terminates
    if (args.contains("--watch") && args.size() >= 5) {
        int idx = args.indexOf("--watch");
        QString filePath = args.at(idx + 1);
        qint64 pid = args.at(idx + 2).toLongLong();
        QByteArray dataKey = QByteArray::fromBase64(args.at(idx + 3).toLatin1());

        // Wait while pid is running
        while (pid > 0 && kill(static_cast<pid_t>(pid), 0) == 0) {
            QThread::msleep(500);
        }

        // Wait briefly in case another process holds the file via fuser
        for (int i = 0; i < 5; ++i) {
            QProcess fuser;
            fuser.start("fuser", {filePath});
            if (fuser.waitForFinished(500)) {
                QString out = QString::fromUtf8(fuser.readAllStandardOutput()).trimmed();
                if (!out.isEmpty()) {
                    QThread::msleep(1000);
                    continue;
                }
            }
            break;
        }

        // Re-encrypt file
        if (QFile::exists(filePath) && !dataKey.isEmpty()) {
            CryptoEngine crypto;
            QByteArray newIv;
            crypto.encryptFile(filePath, dataKey, newIv);

            // Update database
            QString dbPath = QDir::homePath() + "/.config/bubble/vault.db";
            VaultDatabase db;
            if (db.open(dbPath)) {
                VaultEntry entry = db.findByPath(filePath);
                if (entry.id != 0) {
                    entry.encIv = newIv;
                    db.updateEntry(entry);
                    db.removeSession(entry.id);
                }
                db.close();
            }

            // Set permissions to 0000
            QFile file(filePath);
            file.setPermissions(QFileDevice::Permissions{});

            // Extended attribute
            QByteArray pathBa = filePath.toLocal8Bit();
            setxattr(pathBa.constData(), "user.bubble.locked", "1", 1, 0);
        }
        return 0;
    }

    bool allUsers = args.contains("--all-users");

    if (allUsers) {
        std::cout << "Destroying Bubble vaults for all users..." << std::endl;
        QDir homeDir("/home");
        const QStringList userDirs = homeDir.entryList(QDir::Dirs | QDir::NoDotAndDotDot);
        for (const QString &user : userDirs) {
            QString vaultDb = QString("/home/%1/.config/bubble/vault.db").arg(user);
            processVaultDb(vaultDb);
        }
        // Also check root's home
        processVaultDb("/root/.config/bubble/vault.db");
    } else {
        // Current user
        QString configPath = QDir::homePath() + "/.config/bubble/vault.db";
        processVaultDb(configPath);
    }

    std::cout << "Bubble vault cleanup complete." << std::endl;
    return 0;
}
