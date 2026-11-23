package com.makersns.focus

/** Presentation only. Rust owns time, state transitions, and history. */
object TimerPresentation {
    fun time(remainingMs: Long): String {
        val seconds = (remainingMs.coerceAtLeast(0) + 999) / 1000
        return if (seconds >= 3600) "%d:%02d:%02d".format(java.util.Locale.ROOT,
            seconds / 3600, seconds / 60 % 60, seconds % 60)
        else "%02d:%02d".format(java.util.Locale.ROOT, seconds / 60, seconds % 60)
    }
    fun progress(durationMs: Long, remainingMs: Long): Int {
        if (durationMs <= 0) return 0
        return (((durationMs - remainingMs.coerceIn(0, durationMs)) * 1000) / durationMs).toInt()
    }
}
