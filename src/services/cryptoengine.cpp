#include "cryptoengine.h"
#include <QFile>
#include <QDebug>
#include <openssl/evp.h>
#include <openssl/rand.h>
#include <openssl/crypto.h>
#include <argon2.h>
#include <cstring>

CryptoEngine::CryptoEngine(QObject *parent)
    : QObject(parent)
{
}

QByteArray CryptoEngine::generateSalt() const
{
    QByteArray salt;
    salt.resize(SALT_SIZE);
    if (RAND_bytes(reinterpret_cast<unsigned char *>(salt.data()), SALT_SIZE) != 1) {
        qWarning() << "Failed to generate random salt";
        return QByteArray();
    }
    return salt;
}

QByteArray CryptoEngine::generateRandomKey() const
{
    QByteArray key;
    key.resize(KEY_SIZE);
    if (RAND_bytes(reinterpret_cast<unsigned char *>(key.data()), KEY_SIZE) != 1) {
        qWarning() << "Failed to generate random key";
        return QByteArray();
    }
    return key;
}

QByteArray CryptoEngine::generateIv() const
{
    QByteArray iv;
    iv.resize(IV_SIZE);
    if (RAND_bytes(reinterpret_cast<unsigned char *>(iv.data()), IV_SIZE) != 1) {
        qWarning() << "Failed to generate random IV";
        return QByteArray();
    }
    return iv;
}

QByteArray CryptoEngine::hashPassword(const QString &password, QByteArray &saltOut) const
{
    saltOut = generateSalt();
    if (saltOut.isEmpty()) {
        return QByteArray();
    }
    return hashPassword(password, static_cast<const QByteArray &>(saltOut));
}

QByteArray CryptoEngine::hashPassword(const QString &password, const QByteArray &salt) const
{
    if (salt.length() != SALT_SIZE) {
        qWarning() << "Invalid salt size for password hashing";
        return QByteArray();
    }

    QByteArray hash;
    hash.resize(HASH_SIZE);
    QByteArray pwdBytes = password.toUtf8();

    int result = argon2id_hash_raw(ARGON2_TIME_COST, ARGON2_MEMORY_COST, ARGON2_PARALLELISM,
                                   pwdBytes.constData(), pwdBytes.length(),
                                   salt.constData(), salt.length(),
                                   hash.data(), HASH_SIZE);

    if (result != ARGON2_OK) {
        qWarning() << "Argon2id hashing failed:" << argon2_error_message(result);
        return QByteArray();
    }

    return hash;
}

bool CryptoEngine::verifyPassword(const QString &password, const QByteArray &hash, const QByteArray &salt) const
{
    if (hash.length() != HASH_SIZE || salt.length() != SALT_SIZE) {
        return false;
    }

    QByteArray pwdBytes = password.toUtf8();
    QByteArray computedHash;
    computedHash.resize(HASH_SIZE);

    int result = argon2id_hash_raw(ARGON2_TIME_COST, ARGON2_MEMORY_COST, ARGON2_PARALLELISM,
                                   pwdBytes.constData(), pwdBytes.length(),
                                   salt.constData(), salt.length(),
                                   computedHash.data(), HASH_SIZE);

    if (result != ARGON2_OK) {
        qWarning() << "Argon2id hashing failed during verification:" << argon2_error_message(result);
        return false;
    }

    return CRYPTO_memcmp(hash.constData(), computedHash.constData(), HASH_SIZE) == 0;
}

QByteArray CryptoEngine::deriveKey(const QString &password, const QByteArray &salt) const
{
    if (salt.length() != SALT_SIZE) {
        qWarning() << "Invalid salt size for key derivation";
        return QByteArray();
    }

    QByteArray key;
    key.resize(KEY_SIZE);
    QByteArray pwdBytes = password.toUtf8();

    int result = argon2id_hash_raw(ARGON2_TIME_COST, ARGON2_MEMORY_COST, ARGON2_PARALLELISM,
                                   pwdBytes.constData(), pwdBytes.length(),
                                   salt.constData(), salt.length(),
                                   key.data(), KEY_SIZE);

    if (result != ARGON2_OK) {
        qWarning() << "Argon2id key derivation failed:" << argon2_error_message(result);
        return QByteArray();
    }

    return key;
}

QByteArray CryptoEngine::encrypt(const QByteArray &plaintext, const QByteArray &key, QByteArray &ivOut) const
{
    if (key.length() != KEY_SIZE) {
        qWarning() << "Invalid key size for AES-256-GCM encryption";
        return QByteArray();
    }

    ivOut.resize(IV_SIZE);
    if (RAND_bytes(reinterpret_cast<unsigned char *>(ivOut.data()), IV_SIZE) != 1) {
        qWarning() << "Failed to generate random IV";
        return QByteArray();
    }

    EVP_CIPHER_CTX *ctx = EVP_CIPHER_CTX_new();
    if (!ctx) {
        qWarning() << "Failed to create EVP_CIPHER_CTX";
        return QByteArray();
    }

    QByteArray ciphertext;
    ciphertext.resize(plaintext.length() + EVP_MAX_BLOCK_LENGTH + TAG_SIZE);

    int len = 0;
    int ciphertextLen = 0;

    if (1 != EVP_EncryptInit_ex(ctx, EVP_aes_256_gcm(), nullptr, nullptr, nullptr)) {
        qWarning() << "EVP_EncryptInit_ex failed";
        EVP_CIPHER_CTX_free(ctx);
        return QByteArray();
    }

    if (1 != EVP_CIPHER_CTX_ctrl(ctx, EVP_CTRL_GCM_SET_IVLEN, IV_SIZE, nullptr)) {
        qWarning() << "EVP_CIPHER_CTX_ctrl (set IV len) failed";
        EVP_CIPHER_CTX_free(ctx);
        return QByteArray();
    }

    if (1 != EVP_EncryptInit_ex(ctx, nullptr, nullptr, reinterpret_cast<const unsigned char *>(key.constData()), reinterpret_cast<const unsigned char *>(ivOut.constData()))) {
        qWarning() << "EVP_EncryptInit_ex (set key and IV) failed";
        EVP_CIPHER_CTX_free(ctx);
        return QByteArray();
    }

    if (1 != EVP_EncryptUpdate(ctx, reinterpret_cast<unsigned char *>(ciphertext.data()), &len, reinterpret_cast<const unsigned char *>(plaintext.constData()), plaintext.length())) {
        qWarning() << "EVP_EncryptUpdate failed";
        EVP_CIPHER_CTX_free(ctx);
        return QByteArray();
    }
    ciphertextLen = len;

    if (1 != EVP_EncryptFinal_ex(ctx, reinterpret_cast<unsigned char *>(ciphertext.data()) + len, &len)) {
        qWarning() << "EVP_EncryptFinal_ex failed";
        EVP_CIPHER_CTX_free(ctx);
        return QByteArray();
    }
    ciphertextLen += len;

    QByteArray tag;
    tag.resize(TAG_SIZE);
    if (1 != EVP_CIPHER_CTX_ctrl(ctx, EVP_CTRL_GCM_GET_TAG, TAG_SIZE, tag.data())) {
        qWarning() << "EVP_CIPHER_CTX_ctrl (get tag) failed";
        EVP_CIPHER_CTX_free(ctx);
        return QByteArray();
    }

    EVP_CIPHER_CTX_free(ctx);

    ciphertext.resize(ciphertextLen);
    ciphertext.append(tag);

    return ciphertext;
}

QByteArray CryptoEngine::decrypt(const QByteArray &ciphertext, const QByteArray &key, const QByteArray &iv) const
{
    if (key.length() != KEY_SIZE || iv.length() != IV_SIZE) {
        qWarning() << "Invalid key or IV size for AES-256-GCM decryption";
        return QByteArray();
    }

    if (ciphertext.length() < TAG_SIZE) {
        qWarning() << "Ciphertext too short to contain auth tag";
        return QByteArray();
    }

    QByteArray tag = ciphertext.right(TAG_SIZE);
    QByteArray actualCiphertext = ciphertext.left(ciphertext.length() - TAG_SIZE);

    EVP_CIPHER_CTX *ctx = EVP_CIPHER_CTX_new();
    if (!ctx) {
        qWarning() << "Failed to create EVP_CIPHER_CTX";
        return QByteArray();
    }

    QByteArray plaintext;
    plaintext.resize(actualCiphertext.length() + EVP_MAX_BLOCK_LENGTH);

    int len = 0;
    int plaintextLen = 0;

    if (1 != EVP_DecryptInit_ex(ctx, EVP_aes_256_gcm(), nullptr, nullptr, nullptr)) {
        qWarning() << "EVP_DecryptInit_ex failed";
        EVP_CIPHER_CTX_free(ctx);
        return QByteArray();
    }

    if (1 != EVP_CIPHER_CTX_ctrl(ctx, EVP_CTRL_GCM_SET_IVLEN, IV_SIZE, nullptr)) {
        qWarning() << "EVP_CIPHER_CTX_ctrl (set IV len) failed";
        EVP_CIPHER_CTX_free(ctx);
        return QByteArray();
    }

    if (1 != EVP_DecryptInit_ex(ctx, nullptr, nullptr, reinterpret_cast<const unsigned char *>(key.constData()), reinterpret_cast<const unsigned char *>(iv.constData()))) {
        qWarning() << "EVP_DecryptInit_ex (set key and IV) failed";
        EVP_CIPHER_CTX_free(ctx);
        return QByteArray();
    }

    if (1 != EVP_DecryptUpdate(ctx, reinterpret_cast<unsigned char *>(plaintext.data()), &len, reinterpret_cast<const unsigned char *>(actualCiphertext.constData()), actualCiphertext.length())) {
        qWarning() << "EVP_DecryptUpdate failed";
        EVP_CIPHER_CTX_free(ctx);
        return QByteArray();
    }
    plaintextLen = len;

    if (1 != EVP_CIPHER_CTX_ctrl(ctx, EVP_CTRL_GCM_SET_TAG, TAG_SIZE, tag.data())) {
        qWarning() << "EVP_CIPHER_CTX_ctrl (set tag) failed";
        EVP_CIPHER_CTX_free(ctx);
        return QByteArray();
    }

    int ret = EVP_DecryptFinal_ex(ctx, reinterpret_cast<unsigned char *>(plaintext.data()) + len, &len);
    EVP_CIPHER_CTX_free(ctx);

    if (ret > 0) {
        plaintextLen += len;
        plaintext.resize(plaintextLen);
        return plaintext;
    } else {
        qWarning() << "EVP_DecryptFinal_ex failed (authentication failed)";
        return QByteArray();
    }
}

bool CryptoEngine::encryptFile(const QString &filePath, const QByteArray &key, QByteArray &ivOut) const
{
    QFile file(filePath);
    if (!file.open(QIODevice::ReadOnly)) {
        qWarning() << "Failed to open file for reading:" << filePath;
        return false;
    }

    QByteArray plaintext = file.readAll();
    file.close();

    QByteArray ciphertext = encrypt(plaintext, key, ivOut);
    if (ciphertext.isEmpty()) {
        qWarning() << "Failed to encrypt file data";
        return false;
    }

    if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
        qWarning() << "Failed to open file for writing:" << filePath;
        return false;
    }

    if (file.write(ciphertext) != ciphertext.length()) {
        qWarning() << "Failed to write encrypted data to file:" << filePath;
        file.close();
        return false;
    }

    file.close();
    return true;
}

bool CryptoEngine::decryptFile(const QString &filePath, const QByteArray &key, const QByteArray &iv) const
{
    QFile file(filePath);
    if (!file.open(QIODevice::ReadOnly)) {
        qWarning() << "Failed to open file for reading:" << filePath;
        return false;
    }

    QByteArray ciphertext = file.readAll();
    file.close();

    QByteArray plaintext = decrypt(ciphertext, key, iv);
    if (plaintext.isEmpty()) {
        qWarning() << "Failed to decrypt file data (wrong key or corrupted data)";
        return false;
    }

    if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
        qWarning() << "Failed to open file for writing:" << filePath;
        return false;
    }

    if (file.write(plaintext) != plaintext.length()) {
        qWarning() << "Failed to write decrypted data to file:" << filePath;
        file.close();
        return false;
    }

    file.close();
    return true;
}

QByteArray CryptoEngine::encryptKey(const QByteArray &dataKey, const QByteArray &passwordKey) const
{
    QByteArray iv;
    QByteArray ciphertext = encrypt(dataKey, passwordKey, iv);
    if (ciphertext.isEmpty() || iv.length() != IV_SIZE) {
        return QByteArray();
    }
    // Result: [12-byte IV | ciphertext + 16-byte tag]
    QByteArray envelope;
    envelope.reserve(iv.size() + ciphertext.size());
    envelope.append(iv);
    envelope.append(ciphertext);
    return envelope;
}

QByteArray CryptoEngine::decryptKey(const QByteArray &encryptedBlob, const QByteArray &passwordKey) const
{
    if (encryptedBlob.length() < IV_SIZE + TAG_SIZE) {
        qWarning() << "Encrypted key blob too small";
        return QByteArray();
    }

    QByteArray iv = encryptedBlob.left(IV_SIZE);
    QByteArray ciphertext = encryptedBlob.mid(IV_SIZE);
    return decrypt(ciphertext, passwordKey, iv);
}

bool CryptoEngine::shredFile(const QString &filePath)
{
    QFile file(filePath);
    if (!file.open(QIODevice::ReadWrite)) {
        qWarning() << "Failed to open file for shredding:" << filePath;
        return false;
    }

    qint64 size = file.size();
    if (size == 0) {
        file.close();
        return file.remove();
    }

    const int passes = 3;
    const qint64 bufferSize = 1024 * 1024; // 1MB chunks
    QByteArray randomData;
    randomData.resize(bufferSize);

    for (int pass = 0; pass < passes; ++pass) {
        if (!file.seek(0)) {
            qWarning() << "Failed to seek to start of file for shredding pass" << pass;
            file.close();
            return false;
        }

        qint64 remaining = size;
        while (remaining > 0) {
            qint64 toWrite = qMin(remaining, bufferSize);
            if (RAND_bytes(reinterpret_cast<unsigned char *>(randomData.data()), toWrite) != 1) {
                qWarning() << "Failed to generate random data for shredding";
                file.close();
                return false;
            }

            if (file.write(randomData.constData(), toWrite) != toWrite) {
                qWarning() << "Failed to write random data during shredding";
                file.close();
                return false;
            }
            remaining -= toWrite;
        }
        file.flush();
    }

    file.close();
    return QFile::remove(filePath);
}
