package com.optimizadorpro.agent

import com.optimizadorpro.agent.sensors.CpuInfo
import com.optimizadorpro.agent.shizuku.requireSafePackage
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class CpuInfoTest {
    private val sample = """
        Load: 6.12 / 5.87 / 5.42
        CPU usage from 61263ms to 1238ms ago (2024-01-01 10:00:00.000 to 2024-01-01 10:01:00.000):
          23% 1234/com.example.game: 20% user + 3% kernel / faults: 100 minor
          11% 987/system_server: 7% user + 4% kernel
          6.5% 4321/com.dropbox.android: 5% user + 1.5% kernel
        41% TOTAL: 30% user + 10% kernel + 0.3% iowait + 0.7% softirq
    """.trimIndent()

    @Test
    fun parsesProcessesAndTotal() {
        val info = CpuInfo.parse(sample)
        assertEquals(41.0, info.totalPct, 0.001)
        assertEquals(3, info.processes.size)
        assertEquals("com.dropbox.android", info.processes[2].name)
        assertEquals(6.5, info.processes[2].cpuPct, 0.001)
        assertTrue(info.processes[1].isSystem)
    }

    @Test(expected = IllegalArgumentException::class)
    fun rejectsShellInjectionInPackageNames() {
        requireSafePackage("com.foo; rm -rf /")
    }

    @Test
    fun acceptsRealPackageNames() {
        assertEquals("com.tencent.ig", requireSafePackage("com.tencent.ig"))
    }
}
