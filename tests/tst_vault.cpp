#include <QTest>
#include <QTemporaryDir>
#include <QFile>
#include <QSignalSpy>
#include "services/cryptoengine.h"
#include "services/vaultdatabase.h"
#include "services/vaultservice.h"

class TestVault : public QObject
{
    Q_OBJECT

private slots:
    void testCryptoEngineHashVerify();
    void testCryptoEngineEncryptDecrypt();
    void testCryptoEngineKeyEnvelope();
    void testCryptoEngineFileEncryptDecrypt();
    void testCryptoEngineShred();
    void testVaultDatabaseCrud();
    void testVaultServiceLockUnlockFile();
    void testVaultServiceChangePassword();
    void testVaultServiceLockUnlockDirectory();
    void testVaultServiceSessionFolder();
    void testVaultServiceSessionFile();
    void testVaultServiceBruteForceRateLimit();
    void testVaultServiceGhostEntryOnRecreation();
    void testVaultServiceTamperDetection();
};

void TestVault::testCryptoEngineHashVerify()
{
    CryptoEngine crypto;
    QByteArray salt;
    QByteArray hash = crypto.hashPassword("SuperSecret123!", salt);
    QVERIFY(!hash.isEmpty());
    QVERIFY(!salt.isEmpty());
    QCOMPARE(salt.length(), CryptoEngine::SALT_SIZE);

    QVERIFY(crypto.verifyPassword("SuperSecret123!", hash, salt));
    QVERIFY(!crypto.verifyPassword("WrongPassword!", hash, salt));
}

void TestVault::testCryptoEngineEncryptDecrypt()
{
    CryptoEngine crypto;
    QByteArray key = crypto.generateRandomKey();
    QCOMPARE(key.length(), CryptoEngine::KEY_SIZE);

    QByteArray plaintext = "Hello, this is confidential data!";
    QByteArray iv;
    QByteArray ciphertext = crypto.encrypt(plaintext, key, iv);
    QVERIFY(!ciphertext.isEmpty());
    QVERIFY(ciphertext != plaintext);
    QCOMPARE(iv.length(), CryptoEngine::IV_SIZE);

    QByteArray decrypted = crypto.decrypt(ciphertext, key, iv);
    QCOMPARE(decrypted, plaintext);

    // Wrong key decrypt should fail
    QByteArray wrongKey = crypto.generateRandomKey();
    QByteArray failed = crypto.decrypt(ciphertext, wrongKey, iv);
    QVERIFY(failed.isEmpty());
}

void TestVault::testCryptoEngineKeyEnvelope()
{
    CryptoEngine crypto;
    QByteArray dataKey = crypto.generateRandomKey();
    QByteArray salt = crypto.generateSalt();
    QByteArray pwKey = crypto.deriveKey("my-passphrase", salt);

    QByteArray encBlob = crypto.encryptKey(dataKey, pwKey);
    QVERIFY(!encBlob.isEmpty());

    QByteArray recoveredKey = crypto.decryptKey(encBlob, pwKey);
    QCOMPARE(recoveredKey, dataKey);

    QByteArray wrongPwKey = crypto.deriveKey("wrong-passphrase", salt);
    QByteArray badKey = crypto.decryptKey(encBlob, wrongPwKey);
    QVERIFY(badKey.isEmpty());
}

void TestVault::testCryptoEngineFileEncryptDecrypt()
{
    QTemporaryDir tempDir;
    QVERIFY(tempDir.isValid());
    QString testFile = tempDir.filePath("test.txt");

    QByteArray originalContent = "Confidential document text that must be kept safe.";
    {
        QFile f(testFile);
        QVERIFY(f.open(QIODevice::WriteOnly));
        f.write(originalContent);
        f.close();
    }

    CryptoEngine crypto;
    QByteArray key = crypto.generateRandomKey();
    QByteArray iv;
    QVERIFY(crypto.encryptFile(testFile, key, iv));

    // Verify content on disk changed
    {
        QFile f(testFile);
        QVERIFY(f.open(QIODevice::ReadOnly));
        QByteArray onDisk = f.readAll();
        QVERIFY(onDisk != originalContent);
        f.close();
    }

    // Decrypt
    QVERIFY(crypto.decryptFile(testFile, key, iv));

    // Verify original content restored
    {
        QFile f(testFile);
        QVERIFY(f.open(QIODevice::ReadOnly));
        QByteArray restored = f.readAll();
        QCOMPARE(restored, originalContent);
        f.close();
    }
}

void TestVault::testCryptoEngineShred()
{
    QTemporaryDir tempDir;
    QVERIFY(tempDir.isValid());
    QString testFile = tempDir.filePath("shred_me.txt");

    {
        QFile f(testFile);
        QVERIFY(f.open(QIODevice::WriteOnly));
        f.write("Data to be securely deleted");
        f.close();
    }

    QVERIFY(QFile::exists(testFile));
    QVERIFY(CryptoEngine::shredFile(testFile));
    QVERIFY(!QFile::exists(testFile));
}

void TestVault::testVaultDatabaseCrud()
{
    QTemporaryDir tempDir;
    QVERIFY(tempDir.isValid());
    QString dbPath = tempDir.filePath("vault.db");

    VaultDatabase db;
    QVERIFY(db.open(dbPath));
    QVERIFY(db.isOpen());

    VaultEntry entry;
    entry.path = "/test/path/file.txt";
    entry.type = "file";
    entry.pwHash = "dummyhash";
    entry.pwSalt = "dummysalt";
    entry.encKey = "enckey";
    entry.encIv = "enciv";
    entry.encSalt = "encsalt";
    entry.originalPerms = "0644";
    entry.lockedAt = 123456789;
    entry.isOwnPassword = true;
    entry.inode = 987654321;

    QVERIFY(db.addEntry(entry));
    QVERIFY(db.hasEntry("/test/path/file.txt"));

    VaultEntry found = db.findByPath("/test/path/file.txt");
    QCOMPARE(found.path, entry.path);
    QCOMPARE(found.type, entry.type);
    QCOMPARE(found.originalPerms, entry.originalPerms);
    QCOMPARE(found.inode, entry.inode);

    entry.pwHash = "updatedhash";
    QVERIFY(db.updateEntry(entry));
    found = db.findByPath("/test/path/file.txt");
    QCOMPARE(found.pwHash, QByteArray("updatedhash"));

    QCOMPARE(db.allLockedPaths().size(), 1);
    QCOMPARE(db.allLockedPaths().first(), QString("/test/path/file.txt"));

    QVERIFY(db.removeEntry("/test/path/file.txt"));
    QVERIFY(!db.hasEntry("/test/path/file.txt"));

    db.close();
}

void TestVault::testVaultServiceLockUnlockFile()
{
    QTemporaryDir tempDir;
    QVERIFY(tempDir.isValid());
    QString configDir = tempDir.filePath("config");
    QString testFile = tempDir.filePath("secret_document.txt");

    QByteArray originalContent = "Top secret blueprint for Bubble file manager!";
    {
        QFile f(testFile);
        QVERIFY(f.open(QIODevice::WriteOnly));
        f.write(originalContent);
        f.close();
    }

    VaultService vault(configDir);
    QVERIFY(!vault.isLocked(testFile));

    // Lock item
    QVERIFY(vault.lockItem(testFile, "Pa$$w0rd123"));
    QVERIFY(vault.isLocked(testFile));

    // Content should not be readable plaintext on disk
    {
        QFile f(testFile);
        // Permissions may be 0000; restore read permission to inspect disk content
        f.setPermissions(QFileDevice::ReadOwner);
        QVERIFY(f.open(QIODevice::ReadOnly));
        QByteArray diskData = f.readAll();
        QVERIFY(diskData != originalContent);
        f.close();
    }

    // Try unlocking with wrong password
    QVERIFY(!vault.unlockItem(testFile, "WrongPassword"));
    QVERIFY(vault.isLocked(testFile));

    // Unlock with correct password
    QVERIFY(vault.unlockItem(testFile, "Pa$$w0rd123"));
    QVERIFY(!vault.isLocked(testFile));

    // Content should now be restored to original plaintext
    {
        QFile f(testFile);
        QVERIFY(f.open(QIODevice::ReadOnly));
        QByteArray restored = f.readAll();
        QCOMPARE(restored, originalContent);
        f.close();
    }
}

void TestVault::testVaultServiceChangePassword()
{
    QTemporaryDir tempDir;
    QVERIFY(tempDir.isValid());
    QString configDir = tempDir.filePath("config");
    QString testFile = tempDir.filePath("change_pass.txt");

    QByteArray content = "Password rotation test";
    {
        QFile f(testFile);
        QVERIFY(f.open(QIODevice::WriteOnly));
        f.write(content);
        f.close();
    }

    VaultService vault(configDir);
    QVERIFY(vault.lockItem(testFile, "InitialPassword"));

    // Changing with wrong current password fails
    QVERIFY(!vault.changePassword(testFile, "WrongPass", "NewPassword"));

    // Changing with correct current password succeeds
    QVERIFY(vault.changePassword(testFile, "InitialPassword", "NewPassword"));

    // Old password no longer works to unlock
    QVERIFY(!vault.unlockItem(testFile, "InitialPassword"));

    // New password unlocks successfully
    QVERIFY(vault.unlockItem(testFile, "NewPassword"));
    QVERIFY(!vault.isLocked(testFile));

    {
        QFile f(testFile);
        QVERIFY(f.open(QIODevice::ReadOnly));
        QCOMPARE(f.readAll(), content);
        f.close();
    }
}

void TestVault::testVaultServiceLockUnlockDirectory()
{
    QTemporaryDir tempDir;
    QVERIFY(tempDir.isValid());
    QString configDir = tempDir.filePath("config");
    QString secretFolder = tempDir.filePath("secret_folder");
    QDir().mkpath(secretFolder);

    QString file1 = secretFolder + "/doc1.txt";
    QString file2 = secretFolder + "/doc2.txt";

    QByteArray c1 = "Secret document one";
    QByteArray c2 = "Secret document two";

    {
        QFile f1(file1);
        QVERIFY(f1.open(QIODevice::WriteOnly));
        f1.write(c1);
        f1.close();

        QFile f2(file2);
        QVERIFY(f2.open(QIODevice::WriteOnly));
        f2.write(c2);
        f2.close();
    }

    VaultService vault(configDir);

    // Lock the directory
    QVERIFY(vault.lockItem(secretFolder, "FolderSecret456"));
    QVERIFY(vault.isLocked(secretFolder));
    QVERIFY(vault.isLocked(file1));
    QVERIFY(vault.isLocked(file2));

    // Files inside must be encrypted
    {
        QFile fDir(secretFolder);
        fDir.setPermissions(QFileDevice::ReadOwner | QFileDevice::ExeOwner);
        QFile f1(file1);
        f1.setPermissions(QFileDevice::ReadOwner);
        QVERIFY(f1.open(QIODevice::ReadOnly));
        QVERIFY(f1.readAll() != c1);
        f1.close();
        fDir.setPermissions(QFileDevice::Permissions{});
    }

    // Unlock directory
    QVERIFY(vault.unlockItem(secretFolder, "FolderSecret456"));
    QVERIFY(!vault.isLocked(secretFolder));
    QVERIFY(!vault.isLocked(file1));
    QVERIFY(!vault.isLocked(file2));

    // Plaintext content restored
    {
        QFile f1(file1);
        QVERIFY(f1.open(QIODevice::ReadOnly));
        QCOMPARE(f1.readAll(), c1);
        f1.close();

        QFile f2(file2);
        QVERIFY(f2.open(QIODevice::ReadOnly));
        QCOMPARE(f2.readAll(), c2);
        f2.close();
    }
}

void TestVault::testVaultServiceSessionFolder()
{
    QTemporaryDir tempDir;
    QVERIFY(tempDir.isValid());
    QString configDir = tempDir.filePath("config");
    QString secretFolder = tempDir.filePath("session_folder");
    QDir().mkpath(secretFolder);

    QString childFile = secretFolder + "/note.txt";
    QByteArray originalText = "Directory child plaintext note";
    {
        QFile f(childFile);
        QVERIFY(f.open(QIODevice::WriteOnly));
        f.write(originalText);
        f.close();
    }

    VaultService vault(configDir);
    QVERIFY(vault.lockItem(secretFolder, "SessionSecret"));
    QVERIFY(vault.isLocked(secretFolder));
    QVERIFY(!vault.isSessionUnlocked(secretFolder));

    // File inside must be encrypted on disk
    {
        QFile fDir(secretFolder);
        fDir.setPermissions(QFileDevice::ReadOwner | QFileDevice::ExeOwner);
        QFile f(childFile);
        f.setPermissions(QFileDevice::ReadOwner);
        QVERIFY(f.open(QIODevice::ReadOnly));
        QVERIFY(f.readAll() != originalText);
        f.close();
        fDir.setPermissions(QFileDevice::Permissions{});
    }

    // Session unlock folder
    QVERIFY(vault.sessionUnlockFolder(secretFolder, "SessionSecret"));
    QVERIFY(vault.isSessionUnlocked(secretFolder));

    // File inside should be decrypted and accessible in session
    {
        QFile f(childFile);
        QVERIFY(f.open(QIODevice::ReadOnly));
        QCOMPARE(f.readAll(), originalText);
        f.close();
    }

    // Relock session
    vault.sessionRelockFolder(secretFolder);
    QVERIFY(!vault.isSessionUnlocked(secretFolder));
    QVERIFY(vault.isLocked(secretFolder));

    // File inside must be encrypted again on disk
    {
        QFile fDir(secretFolder);
        fDir.setPermissions(QFileDevice::ReadOwner | QFileDevice::ExeOwner);
        QFile f(childFile);
        f.setPermissions(QFileDevice::ReadOwner);
        QVERIFY(f.open(QIODevice::ReadOnly));
        QVERIFY(f.readAll() != originalText);
        f.close();
        fDir.setPermissions(QFileDevice::Permissions{});
    }
    // Restore permissions for cleanup
    QFile fDir(secretFolder);
    fDir.setPermissions(QFileDevice::ReadOwner | QFileDevice::WriteOwner | QFileDevice::ExeOwner);
}

void TestVault::testVaultServiceBruteForceRateLimit()
{
    QTemporaryDir tempDir;
    QVERIFY(tempDir.isValid());
    QString configDir = tempDir.filePath("config");
    QString secretFile = tempDir.filePath("brute_doc.txt");

    {
        QFile f(secretFile);
        QVERIFY(f.open(QIODevice::WriteOnly));
        f.write("Some sensitive secret");
        f.close();
    }

    VaultService vault(configDir);
    QVERIFY(vault.lockItem(secretFile, "CorrectPass123"));

    // First 3 failed attempts should NOT trigger lockout delay
    QVERIFY(!vault.sessionUnlockFile(secretFile, "Wrong1"));
    QCOMPARE(vault.getRemainingLockoutSeconds(secretFile), 0);

    QVERIFY(!vault.sessionUnlockFile(secretFile, "Wrong2"));
    QCOMPARE(vault.getRemainingLockoutSeconds(secretFile), 0);

    QVERIFY(!vault.sessionUnlockFile(secretFile, "Wrong3"));
    QCOMPARE(vault.getRemainingLockoutSeconds(secretFile), 0);

    // 4th failed attempt triggers 5s lockout
    QVERIFY(!vault.sessionUnlockFile(secretFile, "Wrong4"));
    int lockout = vault.getRemainingLockoutSeconds(secretFile);
    QVERIFY(lockout > 0);
    QVERIFY(lockout <= 5);

    // Any attempt (even with correct password) during lockout fails immediately
    QVERIFY(!vault.sessionUnlockFile(secretFile, "CorrectPass123"));
}

void TestVault::testVaultServiceSessionFile()
{
    QTemporaryDir tempDir;
    QVERIFY(tempDir.isValid());
    QString configDir = tempDir.filePath("config");
    QString secretFile = tempDir.filePath("session_doc.txt");

    QByteArray originalContent = "Original confidential text";
    {
        QFile f(secretFile);
        QVERIFY(f.open(QIODevice::WriteOnly));
        f.write(originalContent);
        f.close();
    }

    VaultService vault(configDir);
    QVERIFY(vault.lockItem(secretFile, "MySecretPass"));
    QVERIFY(vault.isLocked(secretFile));
    QVERIFY(!vault.isSessionUnlocked(secretFile));

    // File content should be encrypted on disk
    {
        QFile f(secretFile);
        f.setPermissions(QFileDevice::ReadOwner);
        QVERIFY(f.open(QIODevice::ReadOnly));
        QVERIFY(f.readAll() != originalContent);
        f.close();
    }

    // Open file session
    QVERIFY(vault.sessionUnlockFile(secretFile, "MySecretPass"));
    QVERIFY(vault.isLocked(secretFile)); // Item is still locked in the vault!
    QVERIFY(vault.isSessionUnlocked(secretFile)); // But session is active!

    // Plaintext is accessible during session
    {
        QFile f(secretFile);
        QVERIFY(f.open(QIODevice::ReadOnly));
        QCOMPARE(f.readAll(), originalContent);
        f.close();
    }

    // User edits the file while in session
    QByteArray modifiedContent = "Confidential text modified by user";
    {
        QFile f(secretFile);
        QVERIFY(f.open(QIODevice::WriteOnly));
        f.write(modifiedContent);
        f.close();
    }

    // When the file is closed, sessionRelockFile is triggered
    vault.sessionRelockFile(secretFile);
    QVERIFY(vault.isLocked(secretFile));
    QVERIFY(!vault.isSessionUnlocked(secretFile));

    // After relock, content on disk is encrypted again and permissions are 0000
    {
        QFile f(secretFile);
        f.setPermissions(QFileDevice::ReadOwner);
        QVERIFY(f.open(QIODevice::ReadOnly));
        QVERIFY(f.readAll() != modifiedContent);
        f.close();
    }

    // Next time opened with password, the updated content is decrypted
    QVERIFY(vault.sessionUnlockFile(secretFile, "MySecretPass"));
    {
        QFile f(secretFile);
        QVERIFY(f.open(QIODevice::ReadOnly));
        QCOMPARE(f.readAll(), modifiedContent);
        f.close();
    }
}

void TestVault::testVaultServiceGhostEntryOnRecreation()
{
    QTemporaryDir tempDir;
    QVERIFY(tempDir.isValid());
    QString configDir = tempDir.filePath("config");
    QString testFile = tempDir.filePath("ghost_file.txt");

    {
        QFile f(testFile);
        QVERIFY(f.open(QIODevice::WriteOnly));
        f.write("Secret original content");
        f.close();
    }

    VaultService vault(configDir);
    QVERIFY(vault.lockItem(testFile, "Pass123"));
    QVERIFY(vault.isLocked(testFile));

    // Simulate external deletion (e.g. sudo rm ghost_file.txt)
    QFile::remove(testFile);
    QVERIFY(!QFile::exists(testFile));

    // isLocked should detect the file was deleted on disk and return false
    QVERIFY(!vault.isLocked(testFile));

    // Simulate recreation of a new file with the exact same name
    {
        QFile f(testFile);
        QVERIFY(f.open(QIODevice::WriteOnly));
        f.write("Brand new user file with same name");
        f.close();
    }

    // Newly created file should NOT be considered locked
    QVERIFY(!vault.isLocked(testFile));
    QVERIFY(vault.lastError().isEmpty());
}

void TestVault::testVaultServiceTamperDetection()
{
    QTemporaryDir tempDir;
    QVERIFY(tempDir.isValid());
    QString configDir = tempDir.filePath("config");
    QString testFile = tempDir.filePath("tamper_file.txt");

    {
        QFile f(testFile);
        QVERIFY(f.open(QIODevice::WriteOnly));
        f.write("Confidential financial records");
        f.close();
    }

    VaultService vault(configDir);
    QVERIFY(vault.lockItem(testFile, "LockPassword123"));
    QVERIFY(vault.isLocked(testFile));

    // Simulate external tampering (e.g. sudo nano or disk corruption)
    {
        QFile f(testFile);
        f.setPermissions(QFileDevice::WriteOwner | QFileDevice::ReadOwner);
        QVERIFY(f.open(QIODevice::ReadWrite));
        QByteArray data = f.readAll();
        // Corrupt ciphertext bytes
        if (data.size() > 5) {
            data[data.size() - 5] = ~data[data.size() - 5];
        }
        f.seek(0);
        f.write(data);
        f.close();
        f.setPermissions(QFileDevice::Permissions{});
    }

    // Attempting to unlock with correct password should fail due to GCM auth tag mismatch
    QVERIFY(!vault.unlockItem(testFile, "LockPassword123"));
    // Error message must specifically report tampering
    QVERIFY(vault.lastError().contains("Tampering detected"));

    // Attempting to change password on tampered file should also be rejected
    QVERIFY(!vault.changePassword(testFile, "LockPassword123", "NewSecretPass456"));
    QVERIFY(vault.lastError().contains("Tampering detected"));
}

QTEST_MAIN(TestVault)
#include "tst_vault.moc"

