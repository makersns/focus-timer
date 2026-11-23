package com.makersns.focus

import org.junit.Assert.*
import org.junit.Test

class WidgetGeometryTest {
    @Test fun everySkinKeepsItsShapeAndTouchTargetAcrossSupportedBounds() {
        for (skin in WidgetSkin.entries) for (width in listOf(220f, 280f, 337f, 600f))
            for (height in listOf(132f, 180f, 240f, 400f)) {
                val layout = WidgetGeometry.create(skin, width, height)
                assertEquals(skin.artWidth / skin.artHeight,
                    layout.artwork.width / layout.artwork.height, 0.0001f)
                for (box in listOfNotNull(layout.artwork, layout.clock, layout.state, layout.action)) {
                    assertTrue("$skin at $width x $height left", box.left >= 0)
                    assertTrue("$skin at $width x $height top", box.top >= 0)
                    assertTrue("$skin at $width x $height right", box.right <= width + 0.01f)
                    assertTrue("$skin at $width x $height bottom", box.bottom <= height + 0.01f)
                }
                assertTrue(layout.action.width >= 48f)
                assertTrue(layout.action.height >= 48f)
                if (skin == WidgetSkin.CAPSULE) assertTrue(layout.clock.right < layout.action.left)
                else assertTrue(layout.clock.bottom < layout.action.top)
                layout.state?.let { state ->
                    assertTrue("$skin clock/status must not overlap", layout.clock.bottom <= state.top)
                    if (skin != WidgetSkin.CAPSULE) assertTrue(state.bottom < layout.action.top)
                }
            }
    }

    @Test fun compactRoundSkinsPrioritizeTheClockAndFullSizeAction() {
        for (skin in listOf(WidgetSkin.RING, WidgetSkin.TOMATO)) {
            val compact = WidgetGeometry.create(skin, 220f, 132f)
            assertNull(compact.state)
            assertTrue(compact.clockTextDp >= 21f)
            assertTrue(compact.actionVisualDp >= 34f)
            assertNotNull(WidgetGeometry.create(skin, 300f, 300f).state)
        }
    }

    @Test fun missingLegacyStyleUsesRingAndValidChoicesStayDistinct() {
        assertEquals(WidgetSkin.RING, WidgetSkin.from(""))
        assertEquals(WidgetSkin.RING, WidgetSkin.from("ring"))
        assertEquals(WidgetSkin.TOMATO, WidgetSkin.from("tomato"))
        assertEquals(WidgetSkin.CAPSULE, WidgetSkin.from("capsule"))
    }
}
