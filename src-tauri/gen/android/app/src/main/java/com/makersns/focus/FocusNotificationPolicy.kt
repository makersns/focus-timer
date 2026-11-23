package com.makersns.focus

/** Android-free decisions used by the native notification adapter and its boundary tests. */
internal object FocusNotificationPolicy {
    data class Permission(val value: String, val canRequest: Boolean)

    fun permission(requiresRuntimePermission: Boolean, runtimeGranted: Boolean,
                   appEnabled: Boolean, cachedState: String?): Permission {
        if (!requiresRuntimePermission) return Permission(if (appEnabled) "granted" else "denied", false)
        if (runtimeGranted) return Permission("granted", false)
        return when (cachedState) {
            "prompt" -> Permission("prompt", true)
            "prompt-with-rationale" -> Permission("denied", true)
            "denied", "granted" -> Permission("denied", false)
            else -> Permission("unknown", false)
        }
    }

    /**
     * true means the observable settings permit sound, never a promise that it was heard.
     * Priority-only DND has user-defined rules; without a listener we cannot claim its outcome.
     */
    fun soundAllowed(sound: Boolean, appEnabled: Boolean, channelEnabled: Boolean,
                     channelAudible: Boolean, normalRinger: Boolean, volumePositive: Boolean,
                     interruption: String, bypassPriority: Boolean): Boolean? {
        if (!sound || !appEnabled || !channelEnabled || !channelAudible ||
            !normalRinger || !volumePositive) return false
        return when (interruption) {
            "all" -> true
            "none", "alarms" -> false
            "priority" -> if (bypassPriority) true else null
            else -> null
        }
    }
}

/**
 * Defensive in-process guard in addition to Rust's durable, once-only completion transition.
 * It is claimed even when system settings block a reminder: re-opening the app must not replay it.
 */
internal class CompletionReminderGate(private val capacity: Int = 128) {
    private val claimed = LinkedHashSet<String>()
    init { require(capacity > 0) }

    @Synchronized fun claim(runId: String): Boolean {
        if (runId.isBlank() || !claimed.add(runId)) return false
        while (claimed.size > capacity) claimed.remove(claimed.first())
        return true
    }
}
