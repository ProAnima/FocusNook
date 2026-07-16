package com.proanima.securestorage

import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import android.security.keystore.StrongBoxUnavailableException
import java.security.GeneralSecurityException
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

// Общая крипто-логика, отдельно от SecureStoragePlugin (тот же принцип, что и
// AlarmScheduler.kt в tauri-plugin-reminder-alarm) — здесь единственный
// вызывающий код это сам плагин, но разделение всё равно делает шифрование
// тестируемым отдельно от Tauri Invoke-обвязки.
object KeystoreHelper {
  private const val KEYSTORE_PROVIDER = "AndroidKeyStore"
  private const val TRANSFORMATION = "AES/GCM/NoPadding"
  private const val GCM_TAG_LENGTH_BITS = 128

  // Rust-сторона (android_vault_key.rs) матчит это конкретное значение по
  // префиксу, чтобы отличить "ключа больше нет" (можно сгенерировать новый)
  // от любой другой ошибки (нельзя — иначе баг в проводке выглядел бы так же,
  // как настоящая потеря ключа, и тихо стёр бы рабочий vault-key).
  const val KEY_UNAVAILABLE_PREFIX = "secure-storage:key-unavailable:"

  class KeyUnavailableException(message: String, cause: Throwable? = null) :
    Exception(message, cause)

  private fun keyStore(): KeyStore =
    KeyStore.getInstance(KEYSTORE_PROVIDER).apply { load(null) }

  private fun getOrCreateKey(alias: String): SecretKey {
    val store = keyStore()
    (store.getKey(alias, null) as? SecretKey)?.let { return it }

    val keyGenerator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, KEYSTORE_PROVIDER)
    val purposes = KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
    val builder = KeyGenParameterSpec.Builder(alias, purposes)
      .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
      .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
      .setKeySize(256)
      // setUserAuthenticationRequired нарочно не вызывается: этот ключ должен
      // читаться из фонового sync без пользователя на экране. Установить его
      // означало бы, что смена блокировки экрана перманентно инвалидирует ключ
      // (KeyPermanentlyInvalidatedException).
      // setRandomizedEncryptionRequired остаётся default(true) — намеренно,
      // см. encrypt() ниже про то, почему IV нельзя задавать самим.

    // setIsStrongBoxBacked недоступен раньше API 28 — метода не существует, не
    // просто исключение, поэтому SDK_INT-проверка обязательна, try/catch одной
    // ловлей не подменяет её.
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
      try {
        keyGenerator.init(builder.setIsStrongBoxBacked(true).build())
        return keyGenerator.generateKey()
      } catch (strongBoxUnavailable: StrongBoxUnavailableException) {
        // Падаем в софтверный путь ниже.
      } catch (unsupported: GeneralSecurityException) {
        // Некоторые OEM Keystore/TEE реализации бросают не
        // StrongBoxUnavailableException, а что-то более общее здесь.
      }
    }
    keyGenerator.init(builder.setIsStrongBoxBacked(false).build())
    return keyGenerator.generateKey()
  }

  // Ключ с setRandomizedEncryptionRequired(true) (default) не даёт задать свой
  // IV при init — Cipher бросит InvalidAlgorithmParameterException. Значит IV
  // всегда генерируется Keystore'ом внутри init(), а не нами; cipher.iv
  // читается уже после init и уходит обратно в Rust на хранение рядом с
  // ciphertext.
  fun encrypt(alias: String, plaintext: ByteArray): Pair<ByteArray, ByteArray> {
    val cipher = Cipher.getInstance(TRANSFORMATION)
    cipher.init(Cipher.ENCRYPT_MODE, getOrCreateKey(alias))
    val ciphertext = cipher.doFinal(plaintext)
    return Pair(ciphertext, cipher.iv)
  }

  fun decrypt(alias: String, ciphertext: ByteArray, iv: ByteArray): ByteArray {
    val store = keyStore()
    if (!store.containsAlias(alias)) {
      throw KeyUnavailableException("no key for alias $alias")
    }
    val key = store.getKey(alias, null) as? SecretKey
      ?: throw KeyUnavailableException("alias $alias is not a usable secret key")

    val cipher = Cipher.getInstance(TRANSFORMATION)
    try {
      cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(GCM_TAG_LENGTH_BITS, iv))
    } catch (invalidated: KeyPermanentlyInvalidatedException) {
      throw KeyUnavailableException("key for alias $alias was permanently invalidated", invalidated)
    }
    // Ошибка auth-тега здесь (битый/подмененный ciphertext) сознательно НЕ
    // оборачивается в KeyUnavailableException — ключ в этой ветке уже
    // подтверждённо загрузился, значит проблема в данных, не в ключе.
    return cipher.doFinal(ciphertext)
  }
}
