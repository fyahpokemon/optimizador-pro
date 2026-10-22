plugins {
    id("com.android.application")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

// Firma de la versión release.
// - En GitHub Actions: el flujo descodifica el secreto ANDROID_KEYSTORE_BASE64 y exporta estas
//   variables. Con la misma clave siempre, cada APK nueva se instala encima de la anterior.
// - Sin clave: se firma con la clave de depuración (instalable, pero cada compilación en CI
//   tiene una distinta y habría que desinstalar antes de actualizar).
val releaseKeystore: String? = System.getenv("ANDROID_KEYSTORE_PATH")?.takeIf { it.isNotBlank() }

android {
    namespace = "com.optimizadorpro.optimizador_pro"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        applicationId = "com.optimizadorpro.optimizador_pro"
        // El agente usa APIs de Android 10 (estado térmico, estadísticas de uso por paquete).
        minSdk = 29
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName
        // Sabor del agente: "full" (APK con Shizuku, por defecto) o "play" (Google Play).
        missingDimensionStrategy("distribution", System.getenv("AGENT_FLAVOR") ?: "full")
    }

    signingConfigs {
        if (releaseKeystore != null) {
            create("release") {
                storeFile = file(releaseKeystore)
                storePassword = System.getenv("ANDROID_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("ANDROID_KEY_ALIAS")
                keyPassword = System.getenv("ANDROID_KEY_PASSWORD")
            }
        }
    }

    buildTypes {
        release {
            signingConfig =
                if (releaseKeystore != null) signingConfigs.getByName("release") else signingConfigs.getByName("debug")
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

flutter {
    source = "../.."
}
