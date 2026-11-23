package com.makersns.focus

import android.app.ActivityManager
import android.app.ApplicationExitInfo
import android.content.Context
import android.os.Build
import android.os.SystemClock
import android.provider.Settings
import org.json.JSONObject
import java.io.File

object FocusNative {
    init { System.loadLibrary("focus_timer_lib") }

    @JvmStatic
    external fun dispatchNative(
        directory: String, request: String, wallNow: Long, elapsed: Long,
        boot: Long, forcePause: Boolean
    ): String

    private var firstCall = true

    @Synchronized
    fun dispatch(context: Context, request: JSONObject): JSONObject {
        val directory = File(context.filesDir, "focus")
        val forcePause = firstCall && wasExplicitlyStopped(context, File(directory, "focus-state.json"))
        firstCall = false
        val boot = try {
            Settings.Global.getInt(context.contentResolver, Settings.Global.BOOT_COUNT, -1).toLong()
        } catch (_: Exception) { -1L }
        return JSONObject(dispatchNative(
            directory.absolutePath, request.toString(), System.currentTimeMillis(),
            SystemClock.elapsedRealtime(), boot, forcePause
        ))
    }

    private fun wasExplicitlyStopped(context: Context, stateFile: File): Boolean {
        if (Build.VERSION.SDK_INT < 30 || !stateFile.exists()) return false
        return try {
            val manager = context.getSystemService(ActivityManager::class.java)
            val exit = manager.getHistoricalProcessExitReasons(context.packageName, 0, 1).firstOrNull()
            exit != null && exit.reason == ApplicationExitInfo.REASON_USER_REQUESTED &&
                exit.timestamp >= stateFile.lastModified()
        } catch (_: Exception) { false }
    }
}
