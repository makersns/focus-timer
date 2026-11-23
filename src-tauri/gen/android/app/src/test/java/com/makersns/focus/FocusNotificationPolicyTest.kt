package com.makersns.focus

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class FocusNotificationPolicyTest {
    @Test fun firstRequestAndRetryableDenialRemainDistinctFromPermanentDenial() {
        assertEquals(FocusNotificationPolicy.Permission("prompt", true),
            FocusNotificationPolicy.permission(true, false, false, "prompt"))
        assertEquals(FocusNotificationPolicy.Permission("denied", true),
            FocusNotificationPolicy.permission(true, false, false, "prompt-with-rationale"))
        assertEquals(FocusNotificationPolicy.Permission("denied", false),
            FocusNotificationPolicy.permission(true, false, false, "denied"))
    }

    @Test fun currentSystemGrantWinsOverOldDenialAndOldAndroidHasNoRuntimePrompt() {
        assertEquals(FocusNotificationPolicy.Permission("granted", false),
            FocusNotificationPolicy.permission(true, true, true, "denied"))
        assertEquals(FocusNotificationPolicy.Permission("denied", false),
            FocusNotificationPolicy.permission(false, true, false, null))
        assertEquals(FocusNotificationPolicy.Permission("granted", false),
            FocusNotificationPolicy.permission(false, true, true, null))
        assertEquals(FocusNotificationPolicy.Permission("unknown", false),
            FocusNotificationPolicy.permission(true, false, false, null))
    }

    private fun sound(requested: Boolean = true, app: Boolean = true, channel: Boolean = true,
                      audible: Boolean = true, ringer: Boolean = true, volume: Boolean = true,
                      dnd: String = "all", bypass: Boolean = false) =
        FocusNotificationPolicy.soundAllowed(requested, app, channel, audible, ringer, volume, dnd, bypass)

    @Test fun anyConfirmedMutePreventsReportingSoundAsAllowed() {
        assertEquals(true, sound())
        assertEquals(false, sound(requested = false))
        assertEquals(false, sound(app = false))
        assertEquals(false, sound(channel = false))
        assertEquals(false, sound(audible = false))
        assertEquals(false, sound(ringer = false))
        assertEquals(false, sound(volume = false))
    }

    @Test fun doNotDisturbIsRespectedAndAmbiguousPriorityRulesStayUnknown() {
        assertEquals(false, sound(dnd = "none"))
        assertEquals(false, sound(dnd = "alarms"))
        assertNull(sound(dnd = "priority"))
        assertNull(sound(dnd = "unknown"))
        assertEquals(true, sound(dnd = "priority", bypass = true))
        assertEquals(false, sound(requested = false, dnd = "priority", bypass = true))
    }

    @Test fun completionCannotAlertAgainAfterTheSameRunWasConsumed() {
        val gate = CompletionReminderGate()
        assertTrue(gate.claim("focus-run"))
        assertFalse(gate.claim("focus-run"))
        assertTrue(gate.claim("break-run"))
        assertFalse(gate.claim("focus-run"))
        assertFalse(gate.claim("break-run"))
        assertFalse(gate.claim(""))
    }

    @Test fun completionGuardDoesNotGrowWithTheEntireSessionHistory() {
        val gate = CompletionReminderGate(2)
        assertTrue(gate.claim("one"))
        assertTrue(gate.claim("two"))
        assertTrue(gate.claim("three"))
        assertFalse(gate.claim("three"))
        assertFalse(gate.claim("two"))
        // Older rounds are already rejected by Rust's durable state, so memory stays bounded.
        assertTrue(gate.claim("one"))
    }
}
