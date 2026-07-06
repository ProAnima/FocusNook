package com.proanima.reminderalarm

import android.app.AlarmManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Build

// Общая логика планирования, используемая и из ReminderAlarmPlugin (команды
// от React через Rust), и из BootReceiver (пересоздание alarms после
// перезагрузки, раздел 11 ТЗ) — второму нужен только Context, не Activity.
object AlarmScheduler {
  fun canScheduleExact(context: Context): Boolean {
    val alarmManager = context.getSystemService(AlarmManager::class.java)
    return Build.VERSION.SDK_INT < Build.VERSION_CODES.S || alarmManager.canScheduleExactAlarms()
  }

  fun schedule(context: Context, id: String, title: String, triggerAtMillis: Long): Boolean {
    val alarmManager = context.getSystemService(AlarmManager::class.java)
    val pendingIntent = pendingIntentFor(context, id, title)
    val exact = canScheduleExact(context)
    if (exact) {
      alarmManager.setExactAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, triggerAtMillis, pendingIntent)
    } else {
      alarmManager.set(AlarmManager.RTC_WAKEUP, triggerAtMillis, pendingIntent)
    }
    return exact
  }

  fun cancel(context: Context, id: String) {
    val alarmManager = context.getSystemService(AlarmManager::class.java)
    alarmManager.cancel(pendingIntentFor(context, id, ""))
  }

  private fun pendingIntentFor(context: Context, id: String, title: String): PendingIntent {
    val intent = Intent(context, ReminderAlarmReceiver::class.java).apply {
      putExtra(ReminderAlarmReceiver.EXTRA_ID, id)
      putExtra(ReminderAlarmReceiver.EXTRA_TITLE, title)
    }
    return PendingIntent.getBroadcast(
      context,
      id.hashCode(),
      intent,
      PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
    )
  }
}
