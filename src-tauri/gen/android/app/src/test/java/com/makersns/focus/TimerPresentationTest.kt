package com.makersns.focus
import org.junit.Assert.assertEquals
import org.junit.Test

class TimerPresentationTest {
    @Test fun countdownUsesCeilingAndNeverShowsNegativeTime() {
        assertEquals("00:01", TimerPresentation.time(1))
        assertEquals("00:00", TimerPresentation.time(-20_000))
        assertEquals("25:00", TimerPresentation.time(1_500_000))
        assertEquals("3:00:00", TimerPresentation.time(10_800_000))
    }
    @Test fun progressIsBoundedAndRepresentsElapsedTime() {
        assertEquals(0, TimerPresentation.progress(60_000, 70_000))
        assertEquals(500, TimerPresentation.progress(60_000, 30_000))
        assertEquals(1000, TimerPresentation.progress(60_000, -1))
        assertEquals(0, TimerPresentation.progress(0, 0))
    }
}
