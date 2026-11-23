package com.makersns.focus

import android.Manifest
import android.app.Activity
import android.app.AlarmManager
import android.app.NotificationManager
import android.appwidget.AppWidgetManager
import android.content.ComponentName
import android.content.pm.PackageManager
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.ActivityCompat
import app.tauri.annotation.Command
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import org.json.JSONObject
import java.lang.ref.WeakReference

@TauriPlugin(permissions = [
    Permission(strings = [Manifest.permission.POST_NOTIFICATIONS], alias = "postNotification")
])
class FocusPlugin(private val owner: Activity) : Plugin(owner) {
    private var currentActivity = WeakReference(owner)

    private fun notificationSound(invoke: Invoke): Boolean? = try {
        invoke.getArgs().getBoolean("sound")
    } catch (_: Exception) {
        invoke.reject("invalid_request", "invalid_request"); null
    }

    private fun notificationStatus(sound: Boolean): JSObject {
        val app = owner.applicationContext
        val runtimeGranted = Build.VERSION.SDK_INT < 33 || ActivityCompat.checkSelfPermission(
            app, Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED
        val permission = FocusNotificationPolicy.permission(Build.VERSION.SDK_INT >= 33, runtimeGranted,
            app.getSystemService(NotificationManager::class.java).areNotificationsEnabled(),
            if (Build.VERSION.SDK_INT >= 33) getPermissionState("postNotification")?.toString() else null)
        return JSObject.fromJSONObject(FocusNotifications.status(app, sound, permission))
    }

    @Command
    fun getNotificationStatus(invoke: Invoke) {
        val sound = notificationSound(invoke) ?: return
        try { invoke.resolve(notificationStatus(sound)) }
        catch (_: Exception) { invoke.reject("notification_status_failed", "notification_status_failed") }
    }

    @Command
    fun requestNotificationPermission(invoke: Invoke) {
        val sound = notificationSound(invoke) ?: return
        try {
            val status = notificationStatus(sound)
            if (!status.getBoolean("canRequest")) { invoke.resolve(status); return }
            val activity = currentActivity.get()
            if (activity == null || activity.isDestroyed || activity.isFinishing) {
                invoke.reject("notification_permission_unavailable", "notification_permission_unavailable"); return
            }
            requestPermissionForAlias("postNotification", invoke, "notificationPermissionResult")
        } catch (_: Exception) {
            invoke.reject("notification_permission_failed", "notification_permission_failed")
        }
    }

    @PermissionCallback
    private fun notificationPermissionResult(invoke: Invoke) {
        getNotificationStatus(invoke)
    }

    @Command
    fun openNotificationSettings(invoke: Invoke) {
        val sound = notificationSound(invoke) ?: return
        val activity = currentActivity.get()
        if (activity == null || activity.isDestroyed || activity.isFinishing) {
            invoke.reject("notification_settings_unavailable", "notification_settings_unavailable"); return
        }
        activity.runOnUiThread {
            for (intent in FocusNotifications.settingsIntents(activity, sound)) {
                try { activity.startActivity(intent); invoke.resolve(); return@runOnUiThread }
                catch (_: Exception) { }
            }
            invoke.reject("notification_settings_unavailable", "notification_settings_unavailable")
        }
    }

    @Command
    fun testNotification(invoke: Invoke) {
        val sound = notificationSound(invoke) ?: return
        FocusController.testNotification(owner.applicationContext, sound) { result ->
            result.fold(onSuccess = { submitted ->
                try {
                    invoke.resolve(JSObject().put("submitted", submitted)
                        .put("soundRequested", submitted && sound).put("status", notificationStatus(sound)))
                } catch (_: Exception) { invoke.reject("notification_status_failed", "notification_status_failed") }
            }, onFailure = { invoke.reject("notification_test_failed", "notification_test_failed") })
        }
    }

    @Command
    fun execute(invoke: Invoke) {
        val request = try { JSONObject(invoke.getArgs().getString("request")) }
        catch (_: Exception) { invoke.reject("invalid_request", "invalid_request"); return }
        val action = request.optJSONObject("action")?.optString("type")
        val explicitStart = request.optString("type") == "action" && action in setOf("start", "resume")
        FocusController.submit(owner.applicationContext, request, userInitiated = explicitStart) { result ->
            result.fold(
                onSuccess = { invoke.resolve(JSObject.fromJSONObject(it)) },
                onFailure = { invoke.reject(it.message ?: "timer_unavailable", it.message ?: "timer_unavailable") }
            )
        }
    }

    @Command
    fun getTimerStatus(invoke: Invoke) {
        val manager = AppWidgetManager.getInstance(owner)
        invoke.resolve(JSObject().put("exactAlarmAllowed", FocusController.exactAlarmsAllowed(owner))
            .put("canRequestExactAlarm", Build.VERSION.SDK_INT in 31..32)
            .put("notificationsAllowed", owner.getSystemService(NotificationManager::class.java).areNotificationsEnabled())
            .put("widgetCount", manager.getAppWidgetIds(ComponentName(owner, FocusWidgetProvider::class.java)).size))
    }

    @Command
    fun requestExactAlarmPermission(invoke: Invoke) {
        val supported = Build.VERSION.SDK_INT in 31..32
        if (!supported || FocusController.exactAlarmsAllowed(owner)) {
            invoke.resolve(JSObject().put("supported", supported).put("requested", false)); return
        }
        try {
            owner.startActivity(Intent(Settings.ACTION_REQUEST_SCHEDULE_EXACT_ALARM,
                Uri.parse("package:" + owner.packageName)))
            invoke.resolve(JSObject().put("supported", true).put("requested", true))
        } catch (_: Exception) { invoke.reject("permission_settings_unavailable", "permission_settings_unavailable") }
    }

    @Command
    fun requestHomeScreenWidget(invoke: Invoke) {
        val manager = AppWidgetManager.getInstance(owner)
        val supported = Build.VERSION.SDK_INT >= 26 && manager.isRequestPinAppWidgetSupported
        val preview = if (supported) android.os.Bundle().apply {
            FocusController.latest?.let { response ->
                putParcelable(AppWidgetManager.EXTRA_APPWIDGET_PREVIEW, FocusWidgetRenderer.preview(owner, response))
            }
        } else null
        val requested = supported && manager.requestPinAppWidget(
            ComponentName(owner, FocusWidgetProvider::class.java), preview, null)
        invoke.resolve(JSObject().put("supported", supported).put("requested", requested))
    }

    @Command
    fun openApp(invoke: Invoke) {
        owner.startActivity(FocusController.openAppIntent(owner))
        invoke.resolve()
    }

    @Command
    fun openExternal(invoke: Invoke) {
        val url = when (invoke.getArgs().getString("target")) {
            "author" -> "https://sugar.makersns.com"
            "github" -> "https://github.com/makersns/focus-timer"
            "license" -> "https://github.com/makersns/focus-timer/blob/main/LICENSE"
            else -> { invoke.reject("invalid_external_target", "invalid_external_target"); return }
        }
        try { owner.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url))); invoke.resolve() }
        catch (_: Exception) { invoke.reject("open_external_failed", "open_external_failed") }
    }

    override fun onConfigurationChanged(newConfig: android.content.res.Configuration) {
        FocusController.submit(owner.applicationContext, JSONObject().put("type", "snapshot"))
    }

    override fun onResume(activity: AppCompatActivity) {
        currentActivity = WeakReference(activity)
        FocusController.submit(activity.applicationContext, JSONObject().put("type", "snapshot"), userInitiated = true)
    }
    override fun onDestroy(activity: AppCompatActivity) {
        if (currentActivity.get() === activity) currentActivity.clear()
    }

}
