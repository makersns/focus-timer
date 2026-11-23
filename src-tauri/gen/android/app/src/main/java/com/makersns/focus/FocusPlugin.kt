package com.makersns.focus

import android.app.Activity
import android.app.AlarmManager
import android.app.NotificationManager
import android.appwidget.AppWidgetManager
import android.content.ComponentName
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.appcompat.app.AppCompatActivity
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import org.json.JSONObject

@TauriPlugin
class FocusPlugin(private val owner: Activity) : Plugin(owner) {
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
        val requested = supported && manager.requestPinAppWidget(
            ComponentName(owner, FocusWidgetProvider::class.java), null, null)
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
        FocusController.submit(activity.applicationContext, JSONObject().put("type", "snapshot"), userInitiated = true)
    }
}
