#pragma once

#include <QObject>
#include <QByteArray>
#include <QString>

class CryptoEngine : public QObject
{
    Q_OBJECT

public:
    explicit CryptoEngine(QObject *parent = nullptr);

    // Password hashing with Argon2id
    QByteArray hashPassword(const QString &password, QByteArray &saltOut) const;
    QByteArray hashPassword(const QString &password, const QByteArray &salt) const;
    bool verifyPassword(const QString &password, const QByteArray &hash, const QByteArray &salt) const;

    // Key derivation using Argon2id (256-bit key)
    QByteArray deriveKey(const QString &password, const QByteArray &salt) const;

    // AES-256-GCM encryption/decryption
    QByteArray encrypt(const QByteArray &plaintext, const QByteArray &key, QByteArray &ivOut) const;
    QByteArray decrypt(const QByteArray &ciphertext, const QByteArray &key, const QByteArray &iv) const;

    // Encrypt/decrypt file in-place
    bool encryptFile(const QString &filePath, const QByteArray &key, QByteArray &ivOut) const;
    bool decryptFile(const QString &filePath, const QByteArray &key, const QByteArray &iv) const;

    // Key envelope: encrypts dataKey with passwordKey and prepends IV
    QByteArray encryptKey(const QByteArray &dataKey, const QByteArray &passwordKey) const;
    QByteArray decryptKey(const QByteArray &encryptedBlob, const QByteArray &passwordKey) const;

    // Random generators using OpenSSL RAND_bytes
    QByteArray generateRandomKey() const;
    QByteArray generateSalt() const;
    QByteArray generateIv() const;

    // Secure file shredding (3-pass random overwrite then unlink)
    static bool shredFile(const QString &filePath);

    static constexpr int SALT_SIZE = 16;
    static constexpr int KEY_SIZE = 32;    // AES-256
    static constexpr int IV_SIZE = 12;     // GCM nonce
    static constexpr int TAG_SIZE = 16;    // GCM auth tag
    static constexpr int HASH_SIZE = 32;

private:
    static constexpr int ARGON2_TIME_COST = 3;
    static constexpr int ARGON2_MEMORY_COST = 65536; // 64 MB
    static constexpr int ARGON2_PARALLELISM = 4;
};
