package com.makersns.focus

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.media.AudioAttributes
import android.media.AudioManager
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.core.app.NotificationCompat
import org.json.JSONObject

/**
 * One OS-owned completion notification is the only sound source.
 * There is no player, WebView beep, alarm-volume override or Do Not Disturb bypass.
 */
internal object FocusNotifications {
    // Keep the existing audible ID: upgrades must preserve the user's channel settings.
    const val COMPLETION_CHANNEL = "focus_completion"
    const val SILENT_CHANNEL = "focus_completion_silent"
    private const val COMPLETION_NOTIFICATION = 4103
    private const val TEST_NOTIFICATION = 4104
    private val completions = CompletionReminderGate()

    fun channelId(sound: Boolean): String = if (sound) COMPLETION_CHANNEL else SILENT_CHANNEL

    fun createChannels(context: Context) {
        if (Build.VERSION.SDK_INT < 26) return
        val manager = context.getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(NotificationChannel(COMPLETION_CHANNEL,
            context.getString(R.string.focus_completion_channel), NotificationManager.IMPORTANCE_DEFAULT).apply {
            description = context.getString(R.string.focus_completion_channel_description)
            setSound(Settings.System.DEFAULT_NOTIFICATION_URI, AudioAttributes.Builder()
                .setUsage(AudioAttributes.USAGE_NOTIFICATION).setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION).build())
            enableVibration(false)
        })
        manager.createNotificationChannel(NotificationChannel(SILENT_CHANNEL,
            context.getString(R.string.focus_silent_channel), NotificationManager.IMPORTANCE_LOW).apply {
            description = context.getString(R.string.focus_silent_channel_description)
            setSound(null, null); enableVibration(false)
        })
    }

    fun status(context: Context, sound: Boolean, permission: FocusNotificationPolicy.Permission): JSONObject {
        val local = FocusController.localized(context, FocusController.latest?.optJSONObject("snapshot"))
        createChannels(local)
        val manager = context.getSystemService(NotificationManager::class.java)
        val allowed = manager.areNotificationsEnabled()
        val channel = if (Build.VERSION.SDK_INT >= 26) manager.getNotificationChannel(channelId(sound)) else null
        val channelEnabled: Boolean? = if (Build.VERSION.SDK_INT >= 26)
            channel?.let { it.importance != NotificationManager.IMPORTANCE_NONE } else true
        val channelAudible = if (Build.VERSION.SDK_INT >= 26)
            channel != null && channel.sound != null && channel.importance >= NotificationManager.IMPORTANCE_DEFAULT
            else true
        val audio = context.getSystemService(AudioManager::class.java)
        val interruption = when (manager.currentInterruptionFilter) {
            NotificationManager.INTERRUPTION_FILTER_ALL -> "all"
            NotificationManager.INTERRUPTION_FILTER_NONE -> "none"
            NotificationManager.INTERRUPTION_FILTER_ALARMS -> "alarms"
            NotificationManager.INTERRUPTION_FILTER_PRIORITY -> "priority"
            else -> "unknown"
        }
        val soundAllowed = if (sound && allowed && channelEnabled == null) null
            else FocusNotificationPolicy.soundAllowed(sound, allowed, channelEnabled == true,
            channelAudible, audio.ringerMode == AudioManager.RINGER_MODE_NORMAL,
            audio.getStreamVolume(AudioManager.STREAM_NOTIFICATION) > 0, interruption,
            Build.VERSION.SDK_INT >= 26 && channel?.canBypassDnd() == true)
        return JSONObject().put("permission", permission.value).put("canRequest", permission.canRequest)
            .put("notificationsAllowed", allowed).put("channelEnabled", channelEnabled ?: JSONObject.NULL)
            .put("soundAllowed", soundAllowed ?: JSONObject.NULL)
            .put("settingsAvailable", settingsIntents(context, sound).any {
                it.resolveActivity(context.packageManager) != null
            })
    }

    fun settingsIntents(context: Context, sound: Boolean): List<Intent> {
        val intents = mutableListOf<Intent>()
        if (Build.VERSION.SDK_INT >= 26) {
            if (context.getSystemService(NotificationManager::class.java).areNotificationsEnabled()) {
                intents += Intent(Settings.ACTION_CHANNEL_NOTIFICATION_SETTINGS)
                    .putExtra(Settings.EXTRA_APP_PACKAGE, context.packageName)
                    .putExtra(Settings.EXTRA_CHANNEL_ID, channelId(sound))
            }
            intents += Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS)
                .putExtra(Settings.EXTRA_APP_PACKAGE, context.packageName)
        }
        intents += Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.parse("package:" + context.packageName))
        return intents
    }

    fun completion(context: Context, snapshot: JSONObject, record: JSONObject) {
        if (!completions.claim(record.getString("id"))) return
        val preferences = snapshot.getJSONObject("preferences")
        if (!preferences.optBoolean("notificationsEnabled")) return
        val local = FocusController.localized(context, snapshot)
        val focus = record.getString("mode") == "focus"
        try {
            submit(local, preferences.optBoolean("soundEnabled", true),
                local.getString(if (focus) R.string.focus_completed_title else R.string.focus_break_completed_title),
                local.getString(if (focus) R.string.focus_completed_message else R.string.focus_break_completed),
                "focus-completion-" + record.getString("id"), COMPLETION_NOTIFICATION, onlyAlertOnce = true)
        } catch (_: Exception) {
            // Notification delivery must never roll back or repeat an already completed timer.
        }
    }

    fun test(context: Context, sound: Boolean): Boolean {
        val local = FocusController.localized(context, FocusController.latest?.optJSONObject("snapshot"))
        return submit(local, sound, local.getString(R.string.focus_test_notification_title),
            local.getString(R.string.focus_test_notification_message), "focus-notification-test",
            TEST_NOTIFICATION, onlyAlertOnce = false)
    }

    private fun submit(context: Context, sound: Boolean, title: String, message: String,
                       tag: String, id: Int, onlyAlertOnce: Boolean): Boolean {
        val manager = context.getSystemService(NotificationManager::class.java)
        createChannels(context)
        if (!manager.areNotificationsEnabled()) return false
        if (Build.VERSION.SDK_INT >= 26 &&
            manager.getNotificationChannel(channelId(sound))?.importance == NotificationManager.IMPORTANCE_NONE) return false
        val open = PendingIntent.getActivity(context, 4100, FocusController.openAppIntent(context),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        val notification = NotificationCompat.Builder(context, channelId(sound))
            .setSmallIcon(R.drawable.focus_notification_icon).setContentTitle(title).setContentText(message)
            .setStyle(NotificationCompat.BigTextStyle().bigText(message))
            .setCategory(NotificationCompat.CATEGORY_REMINDER).setPriority(NotificationCompat.PRIORITY_DEFAULT)
            .setVisibility(NotificationCompat.VISIBILITY_PRIVATE).setContentIntent(open)
            .setAutoCancel(true).setOnlyAlertOnce(onlyAlertOnce)
            // The channel owns sound on Android 8+. This URI also supports Android 7.
            .setSound(if (sound) Settings.System.DEFAULT_NOTIFICATION_URI else null)
            // Enforce the app's silent choice even if the user edited the silent channel.
            .setSilent(!sound).build()
        return try { manager.notify(tag, id, notification); true }
        catch (_: SecurityException) { false }
    }
}
