package com.proanima.reminderalarm

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

// AlarmManager forgets alarms after a device reboot. Timing metadata is kept
// separately from the encrypted SQLCipher vault so this receiver never needs
// the database key and cannot crash the application process during boot.
class BootReceiver : BroadcastReceiver() {
  override fun onReceive(context: Context, intent: Intent) {
    if (intent.action != Intent.ACTION_BOOT_COMPLETED) return
    AlarmScheduler.restorePending(context)
  }
}
