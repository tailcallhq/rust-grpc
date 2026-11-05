# Solución Técnica Detallada: **Actualizar Todas las Dependencias Obsoletas**  
**Autor:** @mau16ro (Desarrollador Experto)  
**Fecha:** 5 nov 2026  

---  

## 📋 Índice

1. [Resumen del objetivo](#resumen-del-objetivo)  
2. [Requisitos previos](#requisitos-previos)  
3. [Mapa de tecnologías y gestores de paquetes](#mapa-de-tecnologías-y-gestores-de-paquetes)  
4. [Detección de dependencias obsoletas](#detección-de-dependencias-obsoletas)  
5. [Estrategia de actualización por ecosistema](#estrategia-de-actualización-por-ecosistema)  
   - 5.1. **Node.js / npm / Yarn / pnpm**  
   - 5.2. **Python / pip / poetry / conda**  
   - 5.3. **Java / Maven / Gradle**  
   - 5.4. **Ruby / Bundler**  
   - 5.5. **PHP / Composer**  
   - 5.6. **Otros (Docker, CI, etc.)**  
6. [Automatización con Renovate / Dependabot](#automatización-con-renovate--dependabot)  
7. [Flujo de trabajo (Git) y control de versiones](#flujo-de-trabajo-git-y-control-de-versiones)  
8. [Ejecución de pruebas y validación](#ejecución-de-pruebas-y-validación)  
9. [Gestión de cambios de ruptura (breaking changes)](#gestión-de-cambios-de-ruptura-breaking-changes)  
10. [Rollback y plan de contingencia](#rollback-y-plan-de-contingencia)  
11. [Actualización de la documentación y changelog](#actualización-de-la-documentación-y-changelog)  
12. [Monitorización post‑despliegue](#monitorización-post‑despliegue)  
13. [Checklist final](#checklist-final)  
14. [Anexos: Scripts y ejemplos de CI](#anexos-scripts-y-ejemplos-de-ci)  

---  

## 1️⃣ Resumen del objetivo

> **Objetivo:** Garantizar que el proyecto (o varios repositorios) tengan todas sus dependencias actualizadas a la última versión compatible, sin romper la funcionalidad existente y manteniendo la seguridad al día.

Los entregables incluyen:

- Un proceso reproducible y documentado para **detectar** y **actualizar** dependencias.
- Scripts/automations que puedan ejecutarse en local y en CI.
- Un plan de pruebas y rollback.
- Integración con herramientas de gestión automática (Renovate/Dependabot).
- Actualización del **changelog** y la **documentación** del proyecto.

---  

## 2️⃣ Requisitos previos

| Herramienta | Versión mínima | Uso |
|-------------|----------------|-----|
| `git` | 2.40+ | Control de versiones |
| `Node.js` | 20.x LTS | Para npm/yarn/pnpm, scripts de actualización |
| `npm` / `yarn` / `pnpm` | Última versión | Gestor de paquetes de Node |
| `Python` | 3.11+ | Pip/Poetry |
| `pip` | 24.x | Instalación de paquetes Python |
| `poetry` | 1.8+ | Gestión de dependencias Python (opcional) |
| `Java` | 21 (JDK) | Maven/Gradle |
| `Maven` | 3.9+ | Build de Java |
| `Gradle` | 8.6+ | Build de Java |
| `Ruby` | 3.3+ | Bundler |
| `Bundler` | 2.4+ | Gestión de gems |
| `PHP` | 8.3+ | Composer |
| `Composer` | 2.7+ | Gestión de paquetes PHP |
| `Docker` | 24.x | Contenedores (si existen Dockerfiles) |
| `GitHub CLI` (`gh`) | 2.38+ | Automatizaciones con PRs |
| `jq` | 1.6+ | Manipular JSON en scripts Bash |
| `curl` / `wget` | — | Descarga de assets |
| **Herramientas de seguridad** | — | `npm audit`, `pip-audit`, `snyk`, `OWASP Dependency-Check` |

> **Tip:** Si el proyecto usa un monorepo, cada sub‑paquete tendrá su propio gestor; el proceso debe iterar sobre todos ellos.

---  

## 3️⃣ Mapa de tecnologías y gestores de paquetes

| Carpeta / Archivo | Gestor | Comentario |
|-------------------|--------|------------|
| `package.json` / `package-lock.json` | npm | Node |
| `yarn.lock` | Yarn | Node |
| `pnpm-lock.yaml` | pnpm | Node |
| `requirements.txt` | pip | Python (versión fija) |
| `pyproject.toml` | Poetry | Python (modern) |
| `environment.yml` | conda | Python (entorno conda) |
| `pom.xml` | Maven | Java |
| `build.gradle` / `build.gradle.kts` | Gradle | Java |
| `Gemfile` / `Gemfile.lock` | Bundler | Ruby |
| `composer.json` / `composer.lock` | Composer | PHP |
| `Dockerfile` | Docker | Imágenes base y paquetes del SO |
| `*.csproj` | NuGet | .NET (si aplica) |
| `go.mod` | go modules | Go (si aplica) |

Identifica **todos** los archivos de lock/manifest antes de iniciar la actualización.  

---  

## 4️⃣ Detección de dependencias obsoletas

### 4.1. Herramientas genéricas

| Lenguaje | Herramienta | Comando |
|----------|-------------|---------|
| Node | `npm outdated` / `yarn outdated` / `pnpm outdated` | `npm outdated --json > outdated-node.json` |
| Python | `pip list --outdated --format=json` | `pip list --outdated --format=json > outdated-pip.json` |
| Python (Poetry) | `poetry show --outdated -T` | `poetry show --outdated -T > outdated-poetry.txt` |
| Java (Maven) | `mvn versions:display-dependency-updates` | `mvn versions:display-dependency-updates -DoutputFile=outdated-maven.txt` |
| Java (Gradle) | `./gradlew dependencyUpdates -Drevision=release` (plugin `com.github.ben-manes.versions`) | |
| Ruby | `bundle outdated --format=json` | `bundle outdated --parseable > outdated-ruby.json` |
| PHP | `composer outdated --format=json` | `composer outdated --format=json > outdated-composer.json` |
| Docker | `docker run --rm -v $(pwd):/app -w /app hadolint/hadolint hadolint Dockerfile` (para versiones base) | |
| .NET | `dotnet list package --outdated --include-transitive` | `dotnet list package --outdated > outdated-dotnet.txt` |

### 4.2. Script maestro de detección (Bash)

```bash
#!/usr/bin/env bash
set -euo pipefail
PROJECT_ROOT=$(git rev-parse --show-toplevel)
cd "$PROJECT_ROOT"

# Node
if [[ -f package.json ]]; then
  npm outdated --json > outdated-node.json || true
fi

# Python (pip)
if [[ -f requirements.txt ]]; then
  pip list --outdated --format=json > outdated-pip.json || true
fi

# Poetry
if [[ -f pyproject.toml ]]; then
  poetry show --outdated -T > outdated-poetry.txt || true
fi

# Maven
if [[ -f pom.xml ]]; then
  mvn versions:display-dependency-updates -DoutputFile=outdated-maven.txt > /dev/null
fi

# Gradle
if [[ -f build.gradle ]] || [[ -f build.gradle.kts ]]; then
  ./gradlew dependencyUpdates -Drevision=release -DoutputFile=outdated-gradle.txt > /dev/null
fi

# Ruby
if [[ -f Gemfile ]]; then
  bundle outdated --parseable > outdated-ruby.txt || true
fi

# PHP
if [[ -f composer.json ]]; then
  composer outdated --format=json > outdated-composer.json || true
fi

echo "✅ Detección completada. Archivos de reporte generados."
```

> **Resultado esperado:** Un conjunto de archivos `outdated-*.{json,txt}` que describen las versiones actuales y las disponibles.

---  

## 5️⃣ Estrategia de actualización por ecosistema

> **Principio:** Primero **actualizar en modo no‑ruptura** (minor/patch). Si el proyecto requiere una versión mayor, crear una rama separada y validar exhaustivamente antes de mezclar a `main`.

### 5.1. Node.js (npm / Yarn / pnpm)

#### 5.1.1. Herramienta recomendada: **`npm-check-updates` (ncu)**  

```bash
# Instalar globalmente (una sola vez)
npm i -g npm-check-updates
```

#### 5.1.2. Flujo de actualización

```bash
# 1️⃣ Actualiza solo versiones semver compatibles (patch + minor)
ncu -u --target=minor   # o --target=latest para forzar mayor
npm install

# 2️⃣ Ejecuta pruebas
npm test   # o el script definido en package.json

# 3️⃣ Si todo OK, commit & push
git add package.json package-lock.json
git commit -m "chore: bump Node dependencies (minor/patch)"
git push origin HEAD
```

#### 5.1.3. Opciones avanzadas

| Opción | Uso |
|--------|-----|
| `--reject <pkg>` | Excluir paquetes problemáticos (p.ej. `webpack`) |
| `--jsonAll` | Obtener JSON completo de cambios (útil para CI) |
| `--filter <pkg>` | Actualizar solo un subconjunto |

#### 5.1.4. Monorepo (Lerna / Nx)

```bash
# Lerna
npx lerna exec -- ncu -u --target=minor
npx lerna run bootstrap   # reinstala deps en todos los paquetes
```

---

### 5.2. Python (pip / Poetry)

#### 5.2.1. Pip

```bash
pip install --upgrade pip-tools
# Genera un nuevo requirements.txt con versiones más recientes
pip-compile --upgrade requirements.in
pip-sync                         # instala los cambios
```

#### 5.2.2. Poetry

```bash
# Actualiza solo minor/patch
poetry update

# Si queremos forzar mayor:
poetry add <package>@latest
```

#### 5.2.3. Validación

```bash
pytest -q
# O el framework de tests que use el proyecto
```

---

### 5.3. Java (Maven / Gradle)

#### 5.3.1. Maven

```bash
# Plugin versions-maven-plugin
mvn versions:use-latest-releases -Dincludes=groupId:artifactId
# O actualizar todas (cuidado con breaking changes)
mvn versions:use-latest-versions

# Verifica que el build siga pasando
mvn clean verify
```

#### 5.3.2. Gradle (plugin `ben-manes.versions`)

```bash
# Añadir plugin en build.gradle (si no está)
plugins {
    id "com.github.ben-manes.versions" version "0.51.0"
}

# Ejecutar reporte
./gradlew dependencyUpdates -Drevision=release

# Actualizar dependencias (solo minor/patch)
./gradlew useLatestVersions -DallowMajorUpdates=false
# Para mayor: -DallowMajorUpdates=true

# Verificar
./gradlew clean build
```

---

### 5.4. Ruby (Bundler)

```bash
bundle outdated            # listado
bundle update              # actualiza todo a la versión más reciente compatible
# O actualizar sólo gemas específicas
bundle update rails
```

Ejecuta el suite de pruebas con `rake test` o `rspec`.

---

### 5.5. PHP (Composer)

```bash
composer outdated           # listado
composer update             # actualiza todo (respecta ^ y ~)
# Si quieres forzar mayor:
composer require vendor/package:^2.0
```

Ejecuta pruebas (`phpunit`) y verifica que el autoload se haya regenerado.

---

### 5.6. Otros (Docker, CI, etc.)

- **Dockerfile**: Revisa imágenes base (`FROM node:14` → `node:20`). Usa `hadolint` o `dockerfilelint` y actualiza tags.
- **CI/CD**: Si la pipeline usa versiones fijas de herramientas (p.ej. `node:12`), actualízalas en los archivos