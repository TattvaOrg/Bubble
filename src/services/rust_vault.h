#pragma once

#include <cstdint>
#include <cstddef>

extern "C" {
    void *bubble_vault_new(const char *config_dir);
    void bubble_vault_free(void *vault);
    bool bubble_vault_lock_item(void *vault, const char *path, const char *password);
    bool bubble_vault_unlock_item(void *vault, const char *path, const char *password);
    bool bubble_vault_is_locked(void *vault, const char *path);
    bool bubble_vault_is_session_unlocked(void *vault, const char *path);
    bool bubble_vault_session_unlock_folder(void *vault, const char *path, const char *password);
    bool bubble_vault_session_relock_folder(void *vault, const char *path);
    bool bubble_vault_session_unlock_file(void *vault, const char *path, const char *password);
    bool bubble_vault_session_relock_file(void *vault, const char *path);
    bool bubble_vault_change_password(void *vault, const char *path, const char *old_pass, const char *new_pass);
    uint64_t bubble_vault_get_lockout_seconds(void *vault, const char *path);
    bool bubble_vault_last_error(void *vault, char *out_buf, size_t max_len);
    void bubble_vault_clear_last_error(void *vault);
    bool bubble_vault_shred_file(const char *path);
    bool bubble_vault_has_own_password(void *vault, const char *path);
    void bubble_vault_all_locked_paths(void *vault, void (*callback)(const char *path, void *user_data), void *user_data);
    void bubble_vault_relock_all_sessions(void *vault);
    bool bubble_vault_get_session_data_key(void *vault, const char *path, uint8_t *out_key, size_t key_len);
}
