package com.proanima.reminderalarm

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class ScheduleArgs {
  var id: String? = null
  var title: String? = null
  var triggerAtMillis: Long? = null
}

@InvokeArg
class CancelArgs {
  var id: String? = null
}

// Раздел 11 ТЗ: PendingIntent-based setExactAndAllowWhileIdle — единственный
// путь, который однозначно завязан на SCHEDULE_EXACT_ALARM и переживает
// перезапуск процесса (в отличие от OnAlarmListener). BroadcastReceiver
// (ReminderAlarmReceiver) существует независимо от Activity/WebView, поэтому
// сработает, даже если наш процесс к моменту срабатывания уже убит системой.
@TauriPlugin
class ReminderAlarmPlugin(private val activity: Activity) : Plugin(activity) {

  @Command
  fun scheduleExactAlarm(invoke: Invoke) {
    val args = invoke.parseArgs(ScheduleArgs::class.java)
    val id = args.id
    val triggerAtMillis = args.triggerAtMillis
    if (id == null || triggerAtMillis == null) {
      invoke.reject("id и triggerAtMillis обязательны")
      return
    }

    // Раздел 11 ТЗ: если exact alarm запрещён — не молчим, деградируем до
    // inexact (AlarmScheduler сам решает это внутри). Явный degraded-state в
    // UI (health card) — отдельная задача, здесь только факт деградации
    // доступен вызывающей стороне через "exact" в ответе.
    val exact = AlarmScheduler.schedule(activity, id, args.title ?: "", triggerAtMillis)

    val ret = JSObject()
    ret.put("exact", exact)
    invoke.resolve(ret)
  }

  @Command
  fun cancelAlarm(invoke: Invoke) {
    val args = invoke.parseArgs(CancelArgs::class.java)
    val id = args.id
    if (id == null) {
      invoke.reject("id обязателен")
      return
    }
    AlarmScheduler.cancel(activity, id)
    invoke.resolve()
  }

  @Command
  fun canScheduleExactAlarms(invoke: Invoke) {
    val ret = JSObject()
    ret.put("value", AlarmScheduler.canScheduleExact(activity))
    invoke.resolve(ret)
  }

  // Открывает системный экран разрешения — на Android 13+ это единственный
  // способ выдать SCHEDULE_EXACT_ALARM, обычного runtime-диалога нет.
  @Command
  fun requestExactAlarmPermission(invoke: Invoke) {
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
      val intent = Intent(
        Settings.ACTION_REQUEST_SCHEDULE_EXACT_ALARM,
        Uri.parse("package:" + activity.packageName),
      )
      activity.startActivity(intent)
    }
    invoke.resolve()
  }

  @Command
  fun ensureNotificationPermission(invoke: Invoke) {
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
      val granted = ContextCompat.checkSelfPermission(activity, Manifest.permission.POST_NOTIFICATIONS) ==
        PackageManager.PERMISSION_GRANTED
      if (!granted) {
        ActivityCompat.requestPermissions(
          activity,
          arrayOf(Manifest.permission.POST_NOTIFICATIONS),
          REQUEST_CODE_NOTIFICATIONS,
        )
      }
    }
    invoke.resolve()
  }

  companion object {
    private const val REQUEST_CODE_NOTIFICATIONS = 9401
  }
}
