package com.makersns.focus

import android.app.AlarmManager
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.res.Configuration
import android.os.Build
import android.os.SystemClock
import org.json.JSONObject
import java.util.Locale
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledFuture
import java.util.concurrent.TimeUnit

object FocusController {
    private val executor = Executors.newSingleThreadScheduledExecutor()
    @Volatile var latest: JSONObject? = null
        private set
    private var ticker: ScheduledFuture<*>? = null
    private var alarmKey: String? = null
    private var widgetKey: String? = null
    private var notificationKey: String? = null
    const val RUNNING_CHANNEL = "focus_running"
    const val RUNNING_NOTIFICATION = 4101

    fun exactAlarmsAllowed(context: Context): Boolean =
        Build.VERSION.SDK_INT < 31 || context.getSystemService(AlarmManager::class.java).canScheduleExactAlarms()

    fun openAppIntent(context: Context): Intent =
        Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP)

    fun submit(context: Context, request: JSONObject, userInitiated: Boolean = false,
               callback: ((Result<JSONObject>) -> Unit)? = null) {
        val app = context.applicationContext
        executor.execute {
            try {
                val action = request.optJSONObject("action")?.optString("type")
                if ((action == "start" || action == "resume" || request.optString("type") == "next") && !exactAlarmsAllowed(app)) {
                    throw IllegalStateException("exact_alarm_permission_required")
                }
                var response = FocusNative.dispatch(app, request)
                if (!response.optBoolean("ok")) throw IllegalStateException(response.optString("error", "timer_unavailable"))
                if (response.getJSONObject("snapshot").getJSONObject("timer").getString("status") == "running" &&
                    !exactAlarmsAllowed(app)) {
                    response = FocusNative.dispatch(app, JSONObject().put("type", "action")
                        .put("action", JSONObject().put("type", "pause")))
                }
                consume(app, response, userInitiated)
                deliver(callback, Result.success(response.getJSONObject("snapshot")))
            } catch (error: Throwable) {
                // Refresh stale widget/alarm requests without performing another action.
                try {
                    val recovered = FocusNative.dispatch(app, JSONObject().put("type", "snapshot"))
                    if (recovered.optBoolean("ok")) consume(app, recovered, false)
                } catch (_: Throwable) { }
                deliver(callback, Result.failure(IllegalStateException(error.message ?: "timer_unavailable")))
            }
        }
    }

    private fun deliver(callback: ((Result<JSONObject>) -> Unit)?, result: Result<JSONObject>) {
        // A disposed WebView or failed receiver callback must not repeat a native operation.
        try { callback?.invoke(result) } catch (_: Exception) { }
    }

    fun serviceStarted(context: Context) {
        executor.execute {
            if (ticker == null || ticker!!.isCancelled) {
                // Rust uses the elapsed-time deadline; never queue catch-up ticks after Android freezes a process.
                ticker = executor.scheduleWithFixedDelay({
                    submit(context, JSONObject().put("type", "tick"))
                }, 1, 1, TimeUnit.SECONDS)
            }
        }
    }

    fun serviceStopped() {
        executor.execute { ticker?.cancel(false); ticker = null }
    }

    private fun consume(context: Context, response: JSONObject, userInitiated: Boolean) {
        latest = response
        val snapshot = response.getJSONObject("snapshot")
        // Rust has already completed and persisted this round. An unrelated widget or
        // foreground-service refresh failure must not swallow its once-only reminder.
        response.optJSONObject("completed")?.let { FocusNotifications.completion(context, snapshot, it) }
        val timer = snapshot.getJSONObject("timer")
        val running = timer.getString("status") == "running"
        val resolvedLocale = localized(context, snapshot).resources.configuration.locales.toLanguageTags()
        val resolvedTheme = isDark(context, snapshot)
        syncAlarm(context, response)
        val key = listOf(timer.optString("runId"), timer.getString("status"),
            timer.getString("mode"), timer.getLong("durationMs"), response.optLong("deadlineElapsedMs"),
            timer.getString("intent"), snapshot.getJSONObject("preferences").getString("language"),
            snapshot.getJSONObject("preferences").getString("theme"),
            snapshot.getJSONObject("preferences").optString("miniTimerStyle", "ring"), resolvedLocale, resolvedTheme,
            (timer.getLong("durationMs") - timer.getLong("remainingMs")) / 60_000).joinToString("|")
        if (key != widgetKey) {
            widgetKey = key
            FocusWidgetProvider.renderAll(context, response)
        }
        if (running) {
            // Reassert the start for each explicit app action, including a fast pause/resume
            // while Android is still tearing down the previous service instance.
            if (userInitiated) {
                try {
                    val intent = Intent(context, FocusTimerService::class.java).setAction(FocusTimerService.ACTION_SYNC)
                    if (Build.VERSION.SDK_INT >= 26) context.startForegroundService(intent) else context.startService(intent)
                } catch (_: Exception) {
                    val paused = FocusNative.dispatch(context, JSONObject().put("type", "action")
                        .put("action", JSONObject().put("type", "pause")))
                    consume(context, paused, false)
                    throw IllegalStateException("background_service_unavailable")
                }
            }
            val nextKey = timer.optString("runId") + "|" + response.optLong("deadlineElapsedMs") +
                "|" + snapshot.getJSONObject("preferences").getString("language") + "|" + resolvedLocale
            if (FocusTimerService.active && nextKey != notificationKey) {
                notificationKey = nextKey
                context.getSystemService(NotificationManager::class.java)
                    .notify(RUNNING_NOTIFICATION, runningNotification(context, response))
            }
        } else {
            notificationKey = null
            context.stopService(Intent(context, FocusTimerService::class.java))
            context.getSystemService(NotificationManager::class.java).cancel(RUNNING_NOTIFICATION)
        }
    }

    private fun syncAlarm(context: Context, response: JSONObject) {
        val timer = response.getJSONObject("snapshot").getJSONObject("timer")
        val running = timer.getString("status") == "running"
        val deadline = response.optLong("deadlineElapsedMs", 0)
        val nextKey = if (running) timer.getString("runId") + "|" + deadline else null
        if (nextKey == alarmKey) return
        val manager = context.getSystemService(AlarmManager::class.java)
        val intent = Intent(context, FocusAlarmReceiver::class.java).setAction("com.makersns.focus.ALARM")
        if (running) intent.putExtra("runId", timer.getString("runId"))
        val pending = PendingIntent.getBroadcast(context, 4102, intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        manager.cancel(pending)
        if (running && exactAlarmsAllowed(context)) {
            manager.setExactAndAllowWhileIdle(AlarmManager.ELAPSED_REALTIME_WAKEUP,
                deadline.coerceAtLeast(SystemClock.elapsedRealtime()), pending)
        }
        alarmKey = nextKey
    }

    fun localized(context: Context, snapshot: JSONObject?): Context {
        val language = snapshot?.optJSONObject("preferences")?.optString("language", "system") ?: "system"
        if (language == "system") return context
        val config = Configuration(context.resources.configuration)
        config.setLocale(Locale.forLanguageTag(language))
        return context.createConfigurationContext(config)
    }

    fun isDark(context: Context, snapshot: JSONObject): Boolean {
        return when (snapshot.getJSONObject("preferences").getString("theme")) {
            "dark" -> true
            "light" -> false
            else -> (context.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES
        }
    }

    fun modeLabel(context: Context, mode: String): String = context.getString(when(mode) {
        "shortBreak" -> R.string.focus_short_break
        "longBreak" -> R.string.focus_long_break
        else -> R.string.focus_mode
    })

    fun createChannels(context: Context) {
        if (Build.VERSION.SDK_INT >= 26) {
            val manager = context.getSystemService(NotificationManager::class.java)
            manager.createNotificationChannel(NotificationChannel(RUNNING_CHANNEL,
                context.getString(R.string.focus_running_channel), NotificationManager.IMPORTANCE_LOW).apply {
                setSound(null, null); enableVibration(false); setShowBadge(false)
            })
            FocusNotifications.createChannels(context)
        }
    }

    fun runningNotification(context: Context, response: JSONObject? = latest): Notification {
        val snapshot = response?.optJSONObject("snapshot")
        val local = localized(context, snapshot)
        createChannels(local)
        val builder = if (Build.VERSION.SDK_INT >= 26) Notification.Builder(context, RUNNING_CHANNEL)
            else Notification.Builder(context)
        val open = PendingIntent.getActivity(context, 4100, openAppIntent(context),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        builder.setSmallIcon(R.drawable.focus_notification_icon).setContentTitle("Focus")
            .setContentIntent(open).setOngoing(true).setOnlyAlertOnce(true)
            .setCategory(if (Build.VERSION.SDK_INT >= 31) Notification.CATEGORY_STOPWATCH else Notification.CATEGORY_SERVICE)
            .setVisibility(Notification.VISIBILITY_PUBLIC)
        val timer = snapshot?.optJSONObject("timer")
        if (timer != null && timer.optString("status") == "running") {
            builder.setContentText(modeLabel(local, timer.getString("mode")))
                .setWhen(System.currentTimeMillis() + timer.getLong("remainingMs"))
                .setUsesChronometer(true).setChronometerCountDown(true)
            builder.addAction(Notification.Action.Builder(
                android.graphics.drawable.Icon.createWithResource(context, R.drawable.focus_widget_pause),
                local.getString(R.string.focus_pause), FocusWidgetProvider.actionIntent(context, timer, "pause")
            ).build())
        } else builder.setContentText(local.getString(R.string.focus_restoring))
        return builder.build()
    }

    fun testNotification(context: Context, sound: Boolean, callback: (Result<Boolean>) -> Unit) {
        val app = context.applicationContext
        executor.execute {
            // No JNI/tick/preferences operation: testing must not advance or reset a round.
            val result = runCatching { FocusNotifications.test(app, sound) }
            try { callback(result) } catch (_: Exception) { }
        }
    }
}
