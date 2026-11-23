package com.makersns.focus

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.os.Build
import org.json.JSONObject

class FocusWidgetProvider : AppWidgetProvider() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action !in setOf(AppWidgetManager.ACTION_APPWIDGET_UPDATE,
                AppWidgetManager.ACTION_APPWIDGET_OPTIONS_CHANGED,
                AppWidgetManager.ACTION_APPWIDGET_RESTORED,
                AppWidgetManager.ACTION_APPWIDGET_ENABLED)) {
            super.onReceive(context, intent)
            return
        }
        // Keep a broadcast-started process alive until the asynchronous native read/render finishes.
        val pending = goAsync()
        FocusController.submit(context, JSONObject().put("type", "snapshot")) {
            try {
                FocusController.latest?.let { response -> renderAll(context.applicationContext, response) }
            } finally { pending?.finish() }
        }
    }

    companion object {
        fun actionIntent(context: Context, timer: JSONObject, action: String): PendingIntent {
            if ((action == "start" || action == "resume" || action == "next") && !FocusController.exactAlarmsAllowed(context)) {
                return PendingIntent.getActivity(context, 4100, FocusController.openAppIntent(context),
                    PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
            }
            val runIdentity = if (timer.isNull("runId")) "idle" else timer.getString("runId")
            val identity = android.net.Uri.Builder().scheme("focus").authority("timer-action")
                .appendPath(action).appendPath(runIdentity).build()
            val intent = Intent(context, FocusTimerService::class.java)
                .setAction(FocusTimerService.ACTION_WIDGET).setData(identity)
                .putExtra("action", action).putExtra("runId", runIdentity)
            val requestCode = when(action) { "pause" -> 4111; "resume" -> 4112; "next" -> 4114; else -> 4113 }
            return if (Build.VERSION.SDK_INT >= 26) PendingIntent.getForegroundService(context, requestCode, intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
            else PendingIntent.getService(context, requestCode, intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        }

        fun renderAll(context: Context, response: JSONObject) {
            val manager = AppWidgetManager.getInstance(context)
            val ids = manager.getAppWidgetIds(ComponentName(context, FocusWidgetProvider::class.java))
            for (id in ids) manager.updateAppWidget(id,
                FocusWidgetRenderer.forOptions(context, response, manager.getAppWidgetOptions(id)))
        }
    }
}
