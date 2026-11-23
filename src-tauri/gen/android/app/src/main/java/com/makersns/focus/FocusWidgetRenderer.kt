package com.makersns.focus

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.graphics.PathMeasure
import android.graphics.RectF
import android.graphics.Typeface
import android.os.Build
import android.os.Bundle
import android.util.SizeF
import android.util.TypedValue
import android.view.View
import android.widget.RemoteViews
import org.json.JSONObject
import kotlin.math.min
import kotlin.math.roundToInt

/** Native text/actions over original vector artwork. Only the coarse arc is a bitmap mask. */
internal object FocusWidgetRenderer {
    fun forOptions(context: Context, response: JSONObject, options: Bundle): RemoteViews {
        val portrait = SizeF(options.getInt(AppWidgetManager.OPTION_APPWIDGET_MIN_WIDTH, 280).toFloat(),
            options.getInt(AppWidgetManager.OPTION_APPWIDGET_MAX_HEIGHT, 220).toFloat())
        val landscape = SizeF(options.getInt(AppWidgetManager.OPTION_APPWIDGET_MAX_WIDTH, portrait.width.toInt()).toFloat(),
            options.getInt(AppWidgetManager.OPTION_APPWIDGET_MIN_HEIGHT, portrait.height.toInt()).toFloat())
        val mask = progressMask(response)
        if (Build.VERSION.SDK_INT >= 31) {
            @Suppress("DEPRECATION")
            val sizes = options.getParcelableArrayList<SizeF>(AppWidgetManager.OPTION_APPWIDGET_SIZES)
                ?.filter { it.width.isFinite() && it.height.isFinite() && it.width > 0 && it.height > 0 }
                ?.distinct()?.take(8).orEmpty()
            val choices = sizes.ifEmpty { listOf(portrait, landscape).distinct() }
            return RemoteViews(choices.associateWith { render(context, response, it, mask) })
        }
        return RemoteViews(render(context, response, landscape, mask),
            render(context, response, portrait, mask))
    }

    fun preview(context: Context, response: JSONObject): RemoteViews {
        val snapshot = response.getJSONObject("snapshot")
        val timer = snapshot.getJSONObject("timer")
        val prefs = snapshot.getJSONObject("preferences")
        val skin = WidgetSkin.from(prefs.optString("miniTimerStyle", "ring"))
        val theme = prefs.getString("theme")
        val dark = FocusController.isDark(context, snapshot)
        val local = FocusController.localized(context, snapshot)
        val views = RemoteViews(context.packageName, when (skin) {
            WidgetSkin.RING -> R.layout.focus_skin_preview
            WidgetSkin.TOMATO -> R.layout.focus_skin_preview_tomato
            WidgetSkin.CAPSULE -> R.layout.focus_skin_preview_capsule
        })
        views.setImageViewResource(R.id.focus_skin_art, artwork(skin, theme))
        views.setImageViewBitmap(R.id.focus_skin_progress, progressMask(response))
        views.setImageViewResource(R.id.focus_widget_action, button(skin, theme, timer.getString("status") == "running"))
        views.setTextViewText(R.id.focus_widget_time, TimerPresentation.time(timer.getLong("remainingMs")))
        views.setTextViewText(R.id.focus_widget_state, stateText(local, timer))
        if (theme != "system") {
            val ink = if (skin == WidgetSkin.TOMATO) "#FFF8EC" else if (dark) "#FFF9EE" else "#34332D"
            val progress = if (skin == WidgetSkin.TOMATO) {
                if (dark) "#E9AC82" else "#F7D3B6"
            } else if (dark) "#EFB083" else "#E49B69"
            views.setTextColor(R.id.focus_widget_time, Color.parseColor(ink))
            views.setTextColor(R.id.focus_widget_state, Color.parseColor(ink))
            views.setInt(R.id.focus_skin_progress, "setColorFilter", Color.parseColor(progress))
        }
        return views
    }

    private fun stateText(local: Context, timer: JSONObject): String = when (timer.getString("status")) {
        "paused" -> local.getString(R.string.focus_skin_paused)
        "completed" -> local.getString(R.string.focus_skin_completed)
        "running" -> if (timer.getString("mode") == "focus") local.getString(R.string.focus_skin_running)
            else FocusController.modeLabel(local, timer.getString("mode"))
        else -> local.getString(R.string.focus_skin_ready)
    }

    private fun progressMask(response: JSONObject): Bitmap {
        val snapshot = response.getJSONObject("snapshot")
        val timer = snapshot.getJSONObject("timer")
        val skin = WidgetSkin.from(snapshot.getJSONObject("preferences").optString("miniTimerStyle", "ring"))
        val bitmap = Bitmap.createBitmap((skin.artWidth * 1.5f).roundToInt(),
            (skin.artHeight * 1.5f).roundToInt(), Bitmap.Config.ARGB_8888)
        bitmap.density = Bitmap.DENSITY_NONE
        val progress = TimerPresentation.progress(timer.getLong("durationMs"), timer.getLong("remainingMs"))
        if (progress <= 0) return bitmap // No round-cap dot at idle/reset.
        val path = Path().apply {
            when (skin) {
                WidgetSkin.RING -> { addArc(RectF(13f, 13f, 227f, 227f), -90f, 360f); close() }
                WidgetSkin.CAPSULE -> { addArc(RectF(19f, 18f, 75f, 74f), -90f, 360f); close() }
                WidgetSkin.TOMATO -> {
                    moveTo(120f, 67f)
                    cubicTo(176f, 62f, 210f, 96f, 213f, 140f)
                    cubicTo(219f, 195f, 181f, 233f, 120f, 233f)
                    cubicTo(59f, 233f, 22f, 196f, 27f, 140f)
                    cubicTo(29f, 97f, 64f, 64f, 120f, 67f)
                    close()
                }
            }
        }
        val elapsedPath = if (progress >= 1000) path else Path().also { segment ->
            val measure = PathMeasure(path, true)
            measure.getSegment(0f, measure.length * progress / 1000f, segment, true)
        }
        val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = Color.WHITE
            style = Paint.Style.STROKE
            strokeWidth = if (skin == WidgetSkin.CAPSULE) 3.8f else 4.3f
            strokeCap = Paint.Cap.ROUND
            strokeJoin = Paint.Join.ROUND
        }
        Canvas(bitmap).apply { scale(1.5f, 1.5f); drawPath(elapsedPath, paint) }
        return bitmap
    }

    private fun render(context: Context, response: JSONObject, size: SizeF, mask: Bitmap): RemoteViews {
        val snapshot = response.getJSONObject("snapshot")
        val timer = snapshot.getJSONObject("timer")
        val preferences = snapshot.getJSONObject("preferences")
        val skin = WidgetSkin.from(preferences.optString("miniTimerStyle", "ring"))
        val theme = preferences.getString("theme")
        val dark = FocusController.isDark(context, snapshot)
        val local = FocusController.localized(context, snapshot)
        val geometry = WidgetGeometry.create(skin, size.width, size.height)
        // A different root layout ID prevents RemoteViews.reapply from retaining
        // forced text colors/filters when switching back to system resources.
        val views = RemoteViews(context.packageName, when (skin) {
            WidgetSkin.RING -> if (theme == "system") R.layout.focus_skin_ring
                else R.layout.focus_skin_ring_fixed
            WidgetSkin.TOMATO -> if (theme == "system") R.layout.focus_skin_tomato
                else R.layout.focus_skin_tomato_fixed
            WidgetSkin.CAPSULE -> if (theme == "system") R.layout.focus_skin_capsule
                else R.layout.focus_skin_capsule_fixed
        })
        views.setImageViewResource(R.id.focus_skin_art, artwork(skin, theme))
        views.setImageViewBitmap(R.id.focus_skin_progress, mask)
        val running = timer.getString("status") == "running"
        val action = when (timer.getString("status")) {
            "running" -> "pause"; "paused" -> "resume"; "completed" -> "next"; else -> "start"
        }
        val actionLabel = local.getString(when (action) {
            "pause" -> R.string.focus_pause
            "resume" -> R.string.focus_resume
            "next" -> if (timer.getString("mode") != "focus") R.string.focus_next_focus
                else if (snapshot.getInt("cycleCompleted") >= 4) R.string.focus_next_long_break
                else R.string.focus_next_short_break
            else -> R.string.focus_start
        })
        val state = stateText(local, timer)
        val open = PendingIntent.getActivity(context, 4100, FocusController.openAppIntent(context),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        views.setTextViewText(R.id.focus_widget_time, TimerPresentation.time(timer.getLong("remainingMs")))
        views.setTextViewText(R.id.focus_widget_state, state)
        views.setContentDescription(R.id.focus_widget_root,
            listOf("Focus", FocusController.modeLabel(local, timer.getString("mode")), state,
                timer.getString("intent")).filter { it.isNotBlank() }.joinToString(" · "))
        views.setViewVisibility(R.id.focus_widget_clock, if (running) View.VISIBLE else View.GONE)
        views.setViewVisibility(R.id.focus_widget_time, if (running) View.GONE else View.VISIBLE)
        views.setChronometerCountDown(R.id.focus_widget_clock, true)
        views.setChronometer(R.id.focus_widget_clock, response.optLong("deadlineElapsedMs", 0), null, running)
        views.setImageViewResource(R.id.focus_widget_action, button(skin, theme, running))
        views.setContentDescription(R.id.focus_widget_action,
            if (!FocusController.exactAlarmsAllowed(context) && action != "pause")
                local.getString(R.string.focus_exact_permission) else actionLabel)
        views.setOnClickPendingIntent(R.id.focus_widget_action, FocusWidgetProvider.actionIntent(context, timer, action))
        for (id in listOf(R.id.focus_widget_root, R.id.focus_widget_clock,
                R.id.focus_widget_time, R.id.focus_widget_state)) views.setOnClickPendingIntent(id, open)

        // System variants retain resource colors so the launcher can re-inflate them
        // correctly even while this application is idle or its process is absent.
        if (theme != "system") {
            val primary = if (skin == WidgetSkin.TOMATO) "#FFF8EC" else if (dark) "#FFF9EE" else "#34332D"
            val secondary = if (skin == WidgetSkin.TOMATO) "#FFF8EC" else if (dark) "#D3C8B7" else "#72685F"
            val progress = if (skin == WidgetSkin.TOMATO) {
                if (dark) "#E9AC82" else "#F7D3B6"
            } else if (dark) "#EFB083" else "#E49B69"
            views.setTextColor(R.id.focus_widget_clock, Color.parseColor(primary))
            views.setTextColor(R.id.focus_widget_time, Color.parseColor(primary))
            views.setTextColor(R.id.focus_widget_state, Color.parseColor(secondary))
            views.setInt(R.id.focus_skin_progress, "setColorFilter", Color.parseColor(progress))
        }
        val metrics = context.resources.displayMetrics
        fun place(id: Int, box: WidgetBox) {
            views.setViewPadding(id, (box.left * metrics.density).roundToInt().coerceAtLeast(0),
                (box.top * metrics.density).roundToInt().coerceAtLeast(0),
                ((geometry.width - box.right) * metrics.density).roundToInt().coerceAtLeast(0),
                ((geometry.height - box.bottom) * metrics.density).roundToInt().coerceAtLeast(0))
        }
        place(R.id.focus_skin_art_frame, geometry.artwork)
        place(R.id.focus_skin_clock_frame, geometry.clock)
        place(R.id.focus_skin_action_frame, geometry.action)
        views.setViewVisibility(R.id.focus_skin_state_frame, if (geometry.state == null) View.GONE else View.VISIBLE)
        geometry.state?.let { place(R.id.focus_skin_state_frame, it) }
        val padding = ((geometry.action.width - geometry.actionVisualDp) * metrics.density / 2).roundToInt().coerceAtLeast(0)
        views.setViewPadding(R.id.focus_widget_action, padding, padding, padding, padding)
        val sample = TimerPresentation.time(timer.getLong("durationMs"))
        val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            typeface = Typeface.create("sans-serif-medium", Typeface.NORMAL)
            textSize = TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_SP, geometry.clockTextDp, metrics)
        }
        // Fit the longest clock of this round, including 3:00:00. Neither a large
        // accessibility font nor a launcher resize may ellipsize the countdown.
        val availableWidth = (geometry.clock.width - 2f) * metrics.density
        val availableHeight = geometry.clock.height * metrics.density
        val factor = min(1f, min(availableWidth / paint.measureText(sample).coerceAtLeast(1f),
            availableHeight / (paint.fontMetrics.bottom - paint.fontMetrics.top).coerceAtLeast(1f)))
        val textPx = paint.textSize * factor
        views.setTextViewTextSize(R.id.focus_widget_clock, TypedValue.COMPLEX_UNIT_PX, textPx)
        views.setTextViewTextSize(R.id.focus_widget_time, TypedValue.COMPLEX_UNIT_PX, textPx)
        geometry.state?.let { box ->
            paint.typeface = Typeface.create("sans-serif", Typeface.NORMAL)
            paint.textSize = TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_SP, geometry.stateTextDp, metrics)
            val factorForHeight = min(1f, box.height * metrics.density /
                (paint.fontMetrics.bottom - paint.fontMetrics.top).coerceAtLeast(1f))
            views.setTextViewTextSize(R.id.focus_widget_state, TypedValue.COMPLEX_UNIT_PX, paint.textSize * factorForHeight)
        }
        return views
    }

    private fun artwork(skin: WidgetSkin, theme: String): Int = when (skin) {
        WidgetSkin.RING -> when (theme) {
            "light" -> R.drawable.focus_skin_ring_light; "dark" -> R.drawable.focus_skin_ring_dark
            else -> R.drawable.focus_skin_ring_system
        }
        WidgetSkin.TOMATO -> when (theme) {
            "light" -> R.drawable.focus_skin_tomato_light; "dark" -> R.drawable.focus_skin_tomato_dark
            else -> R.drawable.focus_skin_tomato_system
        }
        WidgetSkin.CAPSULE -> when (theme) {
            "light" -> R.drawable.focus_skin_capsule_light; "dark" -> R.drawable.focus_skin_capsule_dark
            else -> R.drawable.focus_skin_capsule_system
        }
    }

    private fun button(skin: WidgetSkin, theme: String, running: Boolean): Int {
        return if (skin == WidgetSkin.TOMATO) when (theme) {
            "light" -> if (running) R.drawable.focus_skin_tomato_pause_light else R.drawable.focus_skin_tomato_play_light
            "dark" -> if (running) R.drawable.focus_skin_tomato_pause_dark else R.drawable.focus_skin_tomato_play_dark
            else -> if (running) R.drawable.focus_skin_tomato_pause_system else R.drawable.focus_skin_tomato_play_system
        } else when (theme) {
            "light" -> if (running) R.drawable.focus_skin_standard_pause_light else R.drawable.focus_skin_standard_play_light
            "dark" -> if (running) R.drawable.focus_skin_standard_pause_dark else R.drawable.focus_skin_standard_play_dark
            else -> if (running) R.drawable.focus_skin_standard_pause_system else R.drawable.focus_skin_standard_play_system
        }
    }
}
