package com.makersns.focus

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.os.Build
import android.util.TypedValue
import android.view.View
import android.widget.RemoteViews
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
            if (ids.isEmpty()) return
            val snapshot = response.getJSONObject("snapshot")
            val timer = snapshot.getJSONObject("timer")
            val local = FocusController.localized(context, snapshot)
            val dark = FocusController.isDark(context, snapshot)
            val followsSystem = snapshot.getJSONObject("preferences").getString("theme") == "system"
            val running = timer.getString("status") == "running"
            val textColor = android.graphics.Color.parseColor(if (dark) "#F8F8F3" else "#373830")
            val secondary = android.graphics.Color.parseColor(if (dark) "#C4C6BB" else "#868381")
            val open = PendingIntent.getActivity(context, 4100, FocusController.openAppIntent(context),
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
            for (id in ids) {
                val views = RemoteViews(context.packageName,
                    if (followsSystem) R.layout.focus_widget_system
                    else if (dark) R.layout.focus_widget_dark else R.layout.focus_widget)
                val action = when(timer.getString("status")) {
                    "running" -> "pause"; "paused" -> "resume"; "completed" -> "next"; else -> "start"
                }
                val label = local.getString(when(action) {
                    "pause" -> R.string.focus_pause
                    "resume" -> R.string.focus_resume
                    "next" -> if (timer.getString("mode") != "focus") R.string.focus_next_focus
                        else if (snapshot.getInt("cycleCompleted") >= 4) R.string.focus_next_long_break
                        else R.string.focus_next_short_break
                    else -> R.string.focus_start
                })
                views.setTextViewText(R.id.focus_widget_title, "Focus · " + FocusController.modeLabel(local, timer.getString("mode")))
                views.setTextViewText(R.id.focus_widget_intent,
                    timer.getString("intent").ifBlank { local.getString(R.string.focus_widget_gentle) })
                views.setTextViewText(R.id.focus_widget_time, TimerPresentation.time(timer.getLong("remainingMs")))
                views.setViewVisibility(R.id.focus_widget_clock, if (running) View.VISIBLE else View.GONE)
                views.setViewVisibility(R.id.focus_widget_time, if (running) View.GONE else View.VISIBLE)
                views.setChronometerCountDown(R.id.focus_widget_clock, true)
                views.setChronometer(R.id.focus_widget_clock, response.optLong("deadlineElapsedMs", 0), null, running)
                views.setImageViewResource(R.id.focus_widget_action,
                    if (running) R.drawable.focus_widget_pause else R.drawable.focus_widget_play)
                views.setContentDescription(R.id.focus_widget_action,
                    if (!FocusController.exactAlarmsAllowed(context) && action != "pause")
                        local.getString(R.string.focus_exact_permission) else label)
                views.setContentDescription(R.id.focus_widget_intent, timer.getString("intent"))
                views.setOnClickPendingIntent(R.id.focus_widget_action, actionIntent(context, timer, action))
                views.setOnClickPendingIntent(R.id.focus_widget_root, open)
                views.setOnClickPendingIntent(R.id.focus_widget_title, open)
                views.setOnClickPendingIntent(R.id.focus_widget_time, open)
                views.setOnClickPendingIntent(R.id.focus_widget_clock, open)
                if (!followsSystem) {
                    views.setTextColor(R.id.focus_widget_clock, textColor)
                    views.setTextColor(R.id.focus_widget_time, textColor)
                    views.setTextColor(R.id.focus_widget_title, secondary)
                    views.setTextColor(R.id.focus_widget_intent, secondary)
                }
                val width = manager.getAppWidgetOptions(id).getInt(AppWidgetManager.OPTION_APPWIDGET_MIN_WIDTH, 320)
                val size = when { width < 260 -> 28f; width < 320 -> 36f; else -> 48f }
                views.setTextViewTextSize(R.id.focus_widget_clock, TypedValue.COMPLEX_UNIT_SP, size)
                views.setTextViewTextSize(R.id.focus_widget_time, TypedValue.COMPLEX_UNIT_SP, size)
                views.setProgressBar(R.id.focus_widget_progress, 1000,
                    TimerPresentation.progress(timer.getLong("durationMs"), timer.getLong("remainingMs")), false)
                manager.updateAppWidget(id, views)
            }
        }
    }
}
