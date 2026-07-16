package com.proanima.securestorage

import android.app.Activity
import android.util.Base64
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

// android.util.Base64 с NO_WRAP, не java.util.Base64 (нужен API 26+, а minSdk
// здесь 24) и не DEFAULT-флаги (вставляют перевод строки каждые 76 символов —
// ~88-символьный base64 от 64-hex-символьного vault-key ловит это гарантированно).
// Тот же алфавит, что у Rust-стороны base64::engine::general_purpose::STANDARD.
private const val BASE64_FLAGS = Base64.NO_WRAP

@InvokeArg
class EncryptArgs {
  var alias: String? = null
  var plaintextBase64: String? = null
}

@InvokeArg
class DecryptArgs {
  var alias: String? = null
  var ciphertextBase64: String? = null
  var ivBase64: String? = null
}

@TauriPlugin
class SecureStoragePlugin(private val activity: Activity) : Plugin(activity) {
  @Command
  fun encrypt(invoke: Invoke) {
    val args = invoke.parseArgs(EncryptArgs::class.java)
    val alias = args.alias
    val plaintextBase64 = args.plaintextBase64
    if (alias.isNullOrBlank() || plaintextBase64 == null) {
      invoke.reject("alias and plaintextBase64 are required")
      return
    }

    try {
      val plaintext = Base64.decode(plaintextBase64, BASE64_FLAGS)
      val (ciphertext, iv) = KeystoreHelper.encrypt(alias, plaintext)
      val ret = JSObject()
      ret.put("ciphertextBase64", Base64.encodeToString(ciphertext, BASE64_FLAGS))
      ret.put("ivBase64", Base64.encodeToString(iv, BASE64_FLAGS))
      invoke.resolve(ret)
    } catch (error: Exception) {
      invoke.reject("secure-storage encrypt failed: ${error.message ?: error.javaClass.simpleName}")
    }
  }

  @Command
  fun decrypt(invoke: Invoke) {
    val args = invoke.parseArgs(DecryptArgs::class.java)
    val alias = args.alias
    val ciphertextBase64 = args.ciphertextBase64
    val ivBase64 = args.ivBase64
    if (alias.isNullOrBlank() || ciphertextBase64 == null || ivBase64 == null) {
      invoke.reject("alias, ciphertextBase64 and ivBase64 are required")
      return
    }

    try {
      val ciphertext = Base64.decode(ciphertextBase64, BASE64_FLAGS)
      val iv = Base64.decode(ivBase64, BASE64_FLAGS)
      val plaintext = KeystoreHelper.decrypt(alias, ciphertext, iv)
      val ret = JSObject()
      ret.put("plaintextBase64", Base64.encodeToString(plaintext, BASE64_FLAGS))
      invoke.resolve(ret)
    } catch (unavailable: KeystoreHelper.KeyUnavailableException) {
      // Специфический префикс, не просто текст — android_vault_key.rs матчит
      // по нему, чтобы отличить "ключа больше нет" от прочих ошибок дешифровки.
      invoke.reject("${KeystoreHelper.KEY_UNAVAILABLE_PREFIX}${unavailable.message}")
    } catch (error: Exception) {
      invoke.reject("secure-storage decrypt failed: ${error.message ?: error.javaClass.simpleName}")
    }
  }
}
