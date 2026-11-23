package com.makersns.focus

import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import org.json.JSONObject

class FocusTimerService : Service() {
    companion object {
        const val ACTION_SYNC = "com.makersns.focus.SYNC"
        const val ACTION_WIDGET = "com.makersns.focus.WIDGET_ACTION"
        @Volatile var active = false
            private set
    }
    override fun onCreate() {
        super.onCreate()
        active = true
        val notification = FocusController.runningNotification(this)
        if (Build.VERSION.SDK_INT >= 34) startForeground(
            FocusController.RUNNING_NOTIFICATION, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE)
        else startForeground(FocusController.RUNNING_NOTIFICATION, notification)
        FocusController.serviceStarted(applicationContext)
    }
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val request = if (intent?.action == ACTION_WIDGET) {
            val action = intent.getStringExtra("action")
            if (action !in listOf("start", "pause", "resume", "next")) {
                stopSelf(); return START_NOT_STICKY
            }
            if (action == "next") JSONObject().put("type", "next")
                .put("expectedRunId", intent.getStringExtra("runId") ?: "")
            else JSONObject().put("type", "action").put("action", JSONObject().put("type", action)).apply {
                intent.getStringExtra("runId")?.let { put("expectedRunId", it) }
            }
        } else JSONObject().put("type", "snapshot")
        FocusController.submit(applicationContext, request)
        return START_STICKY
    }
    override fun onConfigurationChanged(newConfig: android.content.res.Configuration) {
        super.onConfigurationChanged(newConfig)
        FocusController.submit(applicationContext, JSONObject().put("type", "snapshot"))
    }
    override fun onDestroy() {
        active = false
        FocusController.serviceStopped()
        super.onDestroy()
    }
    override fun onBind(intent: Intent?): IBinder? = null
}
