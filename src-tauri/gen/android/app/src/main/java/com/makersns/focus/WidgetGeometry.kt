package com.makersns.focus

import kotlin.math.max
import kotlin.math.min

/** Layout only. No clock, timer state, or session identity is owned by a widget. */
internal enum class WidgetSkin(val artWidth: Float, val artHeight: Float) {
    RING(240f, 240f), TOMATO(240f, 260f), CAPSULE(320f, 92f);
    companion object {
        fun from(value: String) = when (value) {
            "tomato" -> TOMATO
            "capsule" -> CAPSULE
            else -> RING
        }
    }
}

internal data class WidgetBox(val left: Float, val top: Float, val width: Float, val height: Float) {
    val right get() = left + width
    val bottom get() = top + height
    companion object {
        fun centered(x: Float, y: Float, width: Float, height: Float) =
            WidgetBox(x - width / 2, y - height / 2, width, height)
    }
}

internal data class WidgetGeometry(
    val width: Float, val height: Float, val artwork: WidgetBox,
    val clock: WidgetBox, val state: WidgetBox?, val action: WidgetBox,
    val clockTextDp: Float, val stateTextDp: Float, val actionVisualDp: Float
) {
    companion object {
        fun create(skin: WidgetSkin, widthDp: Float, heightDp: Float): WidgetGeometry {
            val width = widthDp.coerceIn(1f, 1600f)
            val height = heightDp.coerceIn(1f, 1600f)
            val gutter = if (width < 260f || height < 190f) 4f else 8f
            val scale = min(max(1f, width - gutter * 2) / skin.artWidth,
                max(1f, height - gutter * 2) / skin.artHeight)
            val artwork = WidgetBox.centered(width / 2, height / 2,
                skin.artWidth * scale, skin.artHeight * scale)
            fun box(x: Float, y: Float, w: Float, h: Float) =
                WidgetBox.centered(artwork.left + x * scale, artwork.top + y * scale, w, h)
            val capsule = skin == WidgetSkin.CAPSULE
            val tomato = skin == WidgetSkin.TOMATO
            val clockSize = ((if (capsule) 38f else 44f) * scale).coerceIn(21f, 64f)
            val stateSize = ((if (capsule) 12f else 14f) * scale).coerceIn(10f, 18f)
            val actionVisual = ((if (capsule) 38f else 40f) * scale).coerceIn(34f, 64f)
            val hit = max(48f, actionVisual + 8f)
            val action = box(if (capsule) 278f else 120f,
                if (capsule) 46f else if (tomato) 207f else 188f, hit, hit)
            val clockWidth = (if (capsule) 145f else if (tomato) 174f else 186f) * scale
            var clock = box(if (capsule) 156.5f else 120f,
                if (capsule) 31f else if (tomato) 133f else 107f,
                clockWidth, clockSize * 1.25f)
            var state: WidgetBox? = box(if (capsule) 156.5f else 120f,
                if (capsule) 68f else if (tomato) 171f else 145f,
                clockWidth, stateSize * 1.4f)
            // A small launcher cell keeps its large, real button and readable clock.
            // The status remains in accessibility text when it cannot fit between them.
            if (!capsule && (clock.bottom + 0.5f > state!!.top || state.bottom + 2f > action.top)) {
                state = null
                clock = box(120f, if (tomato) 120f else 99f, clockWidth, clockSize * 1.25f)
            }
            return WidgetGeometry(width, height, artwork, clock, state, action,
                clockSize, stateSize, actionVisual)
        }
    }
}
