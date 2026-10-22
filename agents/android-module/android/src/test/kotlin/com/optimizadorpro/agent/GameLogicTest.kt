package com.optimizadorpro.agent

import com.optimizadorpro.agent.game.FrameStats
import com.optimizadorpro.agent.game.FrameWindow
import com.optimizadorpro.agent.game.ThermalGovernor
import com.optimizadorpro.agent.game.ThermalGovernor.Reason
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** Mismos casos que los tests de `core/stutter` y `core/profiles` en Rust. */
class GameLogicTest {
    private fun steady(fps: Double, secs: Double) = List((fps * secs).toInt()) { 1000.0 / fps }

    private fun stats(avg: Double, low1: Double, spm: Double = 0.0, cv: Double = 0.05) =
        FrameStats(600, avg, low1, 0, spm, cv)

    // ---------- FrameStats ----------

    @Test
    fun tooFewFrames() = assertNull(FrameStats.analyze(List(10) { 16.6 }))

    @Test
    fun steady60IsStable() {
        val f = FrameStats.analyze(steady(60.0, 10.0))!!
        assertEquals(60.0, f.avgFps, 0.1)
        assertEquals(0, f.stutters)
        assertTrue(f.stable)
    }

    @Test
    fun spikesAreStutters() {
        val ft = steady(60.0, 10.0).toMutableList()
        for (i in 60 until ft.size step 60) ft[i] = 50.0
        val f = FrameStats.analyze(ft)!!
        assertEquals(9, f.stutters)
        assertFalse(f.stable)
    }

    @Test
    fun smallJitterAtHighFpsIsNotStutter() {
        val ft = steady(240.0, 5.0).toMutableList()
        for (i in 100 until ft.size step 100) ft[i] = 9.0
        assertEquals(0, FrameStats.analyze(ft)!!.stutters)
    }

    @Test
    fun oscillatingFpsIsUnstable() {
        val ft = (0 until 20).flatMap { k -> steady(if (k % 2 == 0) 40.0 else 75.0, 0.5) }
        val f = FrameStats.analyze(ft)!!
        assertTrue(f.pacingCv > 0.15)
        assertFalse(f.stable)
    }

    @Test
    fun windowKeepsOnlyLastSeconds() {
        val w = FrameWindow(2.0)
        repeat(600) { w.push(10.0) }
        assertEquals(200, w.stats()!!.frames)
    }

    // ---------- ThermalGovernor ----------

    @Test
    fun sustainableCapBelowLows() {
        assertEquals(45, ThermalGovernor.sustainableCap(41.0))
        assertEquals(60, ThermalGovernor.sustainableCap(56.0))
        assertEquals(30, ThermalGovernor.sustainableCap(15.0))
    }

    @Test
    fun profileCapByMemory() {
        assertEquals(30, ThermalGovernor.profileCapFor(4.0, 60))
        assertEquals(45, ThermalGovernor.profileCapFor(4.0, 120))
        assertEquals(60, ThermalGovernor.profileCapFor(8.0, 120))
        assertNull(ThermalGovernor.profileCapFor(12.0, 120))
    }

    @Test
    fun heatLowersCapStepByStepWithHysteresis() {
        val g = ThermalGovernor(profileCap = 60)
        val d = g.step(0, thermalStatus = 2, headroom = null, stats = null)!!
        assertEquals(45, d.cap)
        assertEquals(Reason.HEAT, d.reason)
        assertNull(g.step(5_000, 2, null, null)) // menos de 10 s: no se toca
        assertEquals(40, g.step(10_000, 2, null, null)!!.cap)
        assertEquals(30, g.step(20_000, 2, null, null)!!.cap)
        assertNull(g.step(30_000, 2, null, null)) // ya en el mínimo
    }

    @Test
    fun heatUsesMeasuredLowsWhenAvailable() {
        val g = ThermalGovernor(profileCap = null)
        assertEquals(60, g.step(0, 2, null, stats(70.0, 56.0, spm = 10.0))!!.cap)
    }

    @Test
    fun headroomActsBeforeTheSystemThrottles() {
        val g = ThermalGovernor(profileCap = 60)
        val d = g.step(0, thermalStatus = 0, headroom = 0.9f, stats = null)!!
        assertEquals(45, d.cap)
        assertEquals(Reason.HEADROOM, d.reason)
    }

    @Test
    fun coolForAMinuteRaisesBackToProfile() {
        val g = ThermalGovernor(profileCap = 60)
        g.step(0, 2, null, null) // 45
        val ok = stats(45.0, 44.0)
        assertNull(g.step(15_000, 0, 0.3f, ok))
        assertNull(g.step(60_000, 0, 0.3f, ok))
        val d = g.step(75_000, 0, 0.3f, ok)!!
        assertEquals(60, d.cap)
        assertEquals(Reason.RECOVERED, d.reason)
        assertNull(g.step(200_000, 0, 0.3f, ok)) // no pasa del perfil
    }

    @Test
    fun withoutProfileCapRecoveryEndsUncapped() {
        val g = ThermalGovernor(profileCap = null, coolBeforeRaiseMs = 0, minChangeMs = 0)
        g.step(0, 2, null, null) // 120
        assertEquals(90, g.step(1, 2, null, null)!!.cap)
        assertEquals(120, g.step(2, 0, null, null)!!.cap)
        assertNull(g.step(3, 0, null, null)!!.cap) // sin tope
        assertNull(g.step(4, 0, null, null))
    }

    @Test
    fun stutterWithoutHeatLowersToSustainable() {
        val g = ThermalGovernor(profileCap = 60)
        val d = g.step(0, 0, 0.5f, stats(58.0, 41.0, spm = 12.0, cv = 0.25))!!
        assertEquals(45, d.cap)
        assertEquals(Reason.STUTTER, d.reason)
    }

    @Test
    fun warmButNotHotHoldsSteady() {
        val g = ThermalGovernor(profileCap = 60)
        g.step(0, 2, null, null) // 45
        // Ni caliente ni frío (margen 0,75): ni baja ni sube.
        assertNull(g.step(100_000, 0, 0.75f, stats(45.0, 44.0)))
    }
}
