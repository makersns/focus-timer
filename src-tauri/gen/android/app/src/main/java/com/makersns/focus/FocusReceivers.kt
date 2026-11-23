package com.makersns.focus

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.PowerManager
import org.json.JSONObject

class FocusAlarmReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val runId = intent.getStringExtra("runId") ?: return
        val pending = goAsync()
        // Bounded handoff only; no wake lock is held throughout the countdown.
        val wake = context.getSystemService(PowerManager::class.java)
            .newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "Focus:complete")
        wake.setReferenceCounted(false)
        wake.acquire(10_000)
        FocusController.submit(context, JSONObject().put("type", "alarm").put("runId", runId)) {
            try { if (wake.isHeld) wake.release() } finally { pending.finish() }
        }
    }
}

class FocusSystemReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action !in setOf(
                Intent.ACTION_BOOT_COMPLETED, Intent.ACTION_MY_PACKAGE_REPLACED,
                Intent.ACTION_TIME_CHANGED, Intent.ACTION_TIMEZONE_CHANGED, Intent.ACTION_LOCALE_CHANGED,
                "android.app.action.SCHEDULE_EXACT_ALARM_PERMISSION_STATE_CHANGED")) return
        val pending = goAsync()
        FocusController.submit(context, JSONObject().put("type", "snapshot")) {
            try {
                FocusController.latest?.let { response -> FocusWidgetProvider.renderAll(context, response) }
            } finally { pending.finish() }
        }
    }
}
