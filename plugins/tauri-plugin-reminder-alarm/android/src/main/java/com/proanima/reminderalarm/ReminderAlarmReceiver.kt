package com.proanima.reminderalarm

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.media.AudioAttributes
import android.media.RingtoneManager
import android.os.Build
import androidx.core.app.NotificationCompat

// Системой AlarmManager, независимо от того, жив ли процесс приложения —
// это и есть проверка исходного вопроса спайка (раздел 26 ТЗ, спайк №3).
class ReminderAlarmReceiver : BroadcastReceiver() {
  override fun onReceive(context: Context, intent: Intent) {
    val id = intent.getStringExtra(EXTRA_ID) ?: return
    val title = intent.getStringExtra(EXTRA_TITLE).orEmpty()

    ensureChannel(context)

    val launchIntent = context.packageManager.getLaunchIntentForPackage(context.packageName)
    val contentIntent = launchIntent?.let {
      PendingIntent.getActivity(
        context,
        id.hashCode(),
        it,
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
      )
    }

    val notification = NotificationCompat.Builder(context, CHANNEL_ID)
      .setSmallIcon(context.applicationInfo.icon)
      .setContentTitle("Напоминание")
      .setContentText(title)
      .setPriority(NotificationCompat.PRIORITY_HIGH)
      .setCategory(NotificationCompat.CATEGORY_ALARM)
      .setAutoCancel(true)
      .apply { if (contentIntent != null) setContentIntent(contentIntent) }
      .build()

    val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
    manager.notify(id.hashCode(), notification)
  }

  private fun ensureChannel(context: Context) {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
    val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
    if (manager.getNotificationChannel(CHANNEL_ID) != null) return

    // Раздел 11 ТЗ: USAGE_ALARM снимает требование foreground-service для
    // фонового аудио при срабатывании через granted SCHEDULE_EXACT_ALARM —
    // задаётся на канале, у отдельных уведомлений это не переопределить.
    val audioAttributes = AudioAttributes.Builder()
      .setUsage(AudioAttributes.USAGE_ALARM)
      .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
      .build()

    val channel = NotificationChannel(CHANNEL_ID, "Напоминания", NotificationManager.IMPORTANCE_HIGH).apply {
      description = "Звуковые напоминания FocusNook"
      setSound(RingtoneManager.getDefaultUri(RingtoneManager.TYPE_ALARM), audioAttributes)
    }
    manager.createNotificationChannel(channel)
  }

  companion object {
    const val EXTRA_ID = "id"
    const val EXTRA_TITLE = "title"
    private const val CHANNEL_ID = "reminders"
  }
}
