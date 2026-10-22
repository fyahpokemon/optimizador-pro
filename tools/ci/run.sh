#!/usr/bin/env bash
# Ejecuta un comando y, si falla, publica sus líneas de error como anotación de GitHub Actions.
# Así el motivo se ve en el resumen del run (y por la API) sin descargar el registro completo.
#   tools/ci/run.sh "<título>" comando args...
set -uo pipefail
title="$1"; shift
log="$(mktemp)"
"$@" 2>&1 | tee "$log"
code=${PIPESTATUS[0]}
if [ "$code" -ne 0 ]; then
  # Líneas útiles: errores de rustc/cargo, Gradle, Dart y Flutter, con algo de contexto.
  # Sin colores ANSI, que ensucian la anotación.
  sed -i -E 's/\x1B\[[0-9;]*[A-Za-z]//g' "$log"
  msg=$(grep -nE -A3 'error(\[E[0-9]+\])?:|^e: |FAILURE|What went wrong|Exception|Error:|error •|warning •|Expected:|Actual:|Which:|EXCEPTION CAUGHT|Test failed|\[E\]|Unhandled|not found|panicked' "$log" \
        | head -n 80 | cut -c1-300)
  [ -z "$msg" ] && msg=$(tail -n 60 "$log" | cut -c1-300)
  # Formato de anotación: los saltos de línea se codifican como %0A.
  enc=$(printf '%s' "$msg" | sed -e 's/%/%25/g' -e 's/\r//g' | awk 'BEGIN{ORS="%0A"} {print}')
  echo "::error title=${title}::${enc}"
fi
rm -f "$log"
exit "$code"
