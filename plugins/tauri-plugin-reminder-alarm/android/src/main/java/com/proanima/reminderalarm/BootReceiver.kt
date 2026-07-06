package com.proanima.reminderalarm

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.database.sqlite.SQLiteDatabase
import java.io.File
import java.text.SimpleDateFormat
import java.util.Locale
import java.util.TimeZone
import org.json.JSONObject

// Раздел 11 ТЗ: "После перезагрузки устройства пересоздавать ближайшие
// alarms" — в отличие от смерти процесса приложения (которую AlarmManager
// переживает сам), перезагрузка устройства сбрасывает все запланированные
// alarms целиком, их нужно расставить заново явно.
//
// БД читаем напрямую из Kotlin, без Rust/приложения — на Android она сейчас
// не зашифрована (раздел 26, спайк №2: bundled-sqlcipher-vendored-openssl не
// собирается под Android с Windows-хоста), поэтому ключ не нужен. Путь
// проверен эмпирически через `adb shell run-as ... find` — Tauri app_data_dir
// на Android резолвится в корень приватных данных приложения (context.dataDir),
// а не в files/ или databases/.
class BootReceiver : BroadcastReceiver() {
  override fun onReceive(context: Context, intent: Intent) {
    if (intent.action != Intent.ACTION_BOOT_COMPLETED) return

    val dbFile = activeVaultFile(context) ?: return
    if (!dbFile.exists()) return

    val db = try {
      SQLiteDatabase.openDatabase(dbFile.path, null, SQLiteDatabase.OPEN_READONLY)
    } catch (e: Exception) {
      return
    }

    db.use { database ->
      database.rawQuery(
        "SELECT id, title, trigger_at_utc FROM reminders WHERE status = 'scheduled'",
        null,
      ).use { cursor ->
        while (cursor.moveToNext()) {
          val id = cursor.getString(0)
          val title = cursor.getString(1)
          val triggerAtUtc = cursor.getString(2)
          val millis = parseIso8601UtcMillis(triggerAtUtc) ?: continue
          AlarmScheduler.schedule(context, id, title, millis)
        }
      }
    }
  }

  // trigger_at_utc всегда приходит из JS Date.prototype.toISOString() —
  // фиксированный формат "YYYY-MM-DDTHH:mm:ss.sssZ" (см. Rust-аналог
  // reminders::parse_trigger_millis в apps/desktop/src-tauri).
  private fun activeVaultFile(context: Context): File? {
    val profilesIndex = File(context.dataDir, "profiles.json")
    if (!profilesIndex.exists()) {
      return File(context.dataDir, "vault.db")
    }

    val activeProfileId = try {
      val parsed = JSONObject(profilesIndex.readText())
      parsed.optString("active_profile_id")
        .ifBlank {
          parsed.optJSONArray("profiles")
            ?.optJSONObject(0)
            ?.optString("id")
            .orEmpty()
        }
    } catch (e: Exception) {
      return null
    }

    if (activeProfileId.isBlank()) return null
    return File(context.dataDir, "vault-$activeProfileId.db")
  }

  private fun parseIso8601UtcMillis(value: String): Long? {
    return try {
      val format = SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'", Locale.US)
      format.timeZone = TimeZone.getTimeZone("UTC")
      format.parse(value)?.time
    } catch (e: Exception) {
      null
    }
  }
}
