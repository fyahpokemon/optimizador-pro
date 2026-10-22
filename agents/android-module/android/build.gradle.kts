// Plugin Flutter de Android: sensores, ajustes estándar y ajustes elevados (Shizuku).
// Sigue la plantilla de plugins de Flutter 3.47 (AGP 9.1, Kotlin 2.4). Con
// android.builtInKotlin=false, Flutter aplica él mismo el plugin de Kotlin a este proyecto.
group = "com.optimizadorpro.agent"
version = "0.1.0"

buildscript {
    val kotlinVersion = "2.4.0"
    repositories {
        google()
        mavenCentral()
    }
    dependencies {
        classpath("com.android.tools.build:gradle:9.1.0")
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:$kotlinVersion")
    }
}

allprojects {
    repositories {
        google()
        mavenCentral()
    }
}

plugins {
    id("com.android.library")
}

android {
    namespace = "com.optimizadorpro.agent"
    compileSdk = 36

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        // ThermalStatus listener y UsageStats por paquete; getThermalHeadroom (30) se comprueba en código.
        minSdk = 29
        consumerProguardFiles("proguard-rules.pro")
    }

    buildFeatures {
        aidl = true
    }

    // Dos sabores: "play" (solo APIs públicas, apto para Google Play) y "full" (APK con Shizuku).
    // La app elige uno con missingDimensionStrategy("distribution", …).
    flavorDimensions += "distribution"
    productFlavors {
        create("play") { dimension = "distribution" }
        create("full") { dimension = "distribution" }
    }

    sourceSets {
        getByName("main") { java.srcDirs("src/main/kotlin") }
        getByName("play") { java.srcDirs("src/play/kotlin") }
        getByName("full") { java.srcDirs("src/full/kotlin") }
        getByName("test") { java.srcDirs("src/test/kotlin") }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

dependencies {
    "fullImplementation"("dev.rikka.shizuku:api:13.1.5")
    "fullImplementation"("dev.rikka.shizuku:provider:13.1.5")
    testImplementation("junit:junit:4.13.2")
}
