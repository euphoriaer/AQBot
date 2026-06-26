# Funktionen

## Chat & Modelle

- **Multi-Provider-Chat** — OpenAI, Claude, Gemini, DeepSeek, Qwen und OpenAI-kompatible Endpunkte mit Base URL, API Path, Headers und Proxy-Regeln verbinden.
- **Provider-Onboarding** — aqbot:// Provider-Links und CC Switch-Import übernehmen Provider-Profile nach Benutzerbestätigung.
- **Modellverwaltung** — Remote-Modelle synchronisieren, Gruppen, Latenz, Fähigkeiten, Kontextlänge, Sampling, Reasoning-Profile und extra_body pro Modell konfigurieren.
- **Gesprächs-Workflows** — Streaming, Denkblöcke, Nachrichtenversionen, Branches, Titelstatus, Komprimierung und Multi-Modell-Vergleich.

## AI Agent

- **Agent-Modus** — Das Modell kann Dateien bearbeiten, Befehle ausführen und Code in einem kontrollierten Workflow analysieren.
- **Berechtigungen** — Standardprüfung, Auto-Accept-Edits oder Vollzugriff mit aktiver Arbeitsverzeichnis-Sandbox.
- **Freigabe und Kosten** — Tool-Aufrufe prüfen, Entscheidungen merken und Tokens/Kosten pro Session verfolgen.

## Skills-Verwaltung

- **Multi-Source-Skill-Verzeichnisse** — AQBot-, Codex-, Claude- und Agents-Skill-Roots verwalten, inklusive `~/.aqbot/skills`, `~/.codex/skills`, `~/.claude/skills` und `~/.agents/skills`.
- **Meine Skills** — Nach Quelle filtern, aktivieren/deaktivieren, Details ansehen, Namen kopieren, Verzeichnis öffnen und deinstallieren.
- **Skill-Gruppen und Installationsziele** — Skills per group einklappen, gesammelt aktivieren/deaktivieren, Gruppenordner öffnen, ganze Gruppen deinstallieren und aus `owner/repo` oder GitHub-URLs in ein Ziel installieren.
- **Marketplace** — skills.sh- und GitHub-Quellen durchsuchen, Details ansehen, zu GitHub wechseln und Installationsstatus sehen.

## Inhaltsrendering

- **Markdown und Mathematik** — Markdown, Code, Tabellen, Aufgabenlisten und LaTeX in Streaming-Gesprächen rendern.
- **Code, Diagramme, Artifacts** — Monaco, Mermaid, D2 und Artifact-Panel für Code, Markdown-Notizen, Reports und Vorschauen.
- **HTML-Fragmente** — Generierte HTML-Fragmente sicher anzeigen, inklusive aktueller Streaming-Stabilitätsfixes.

## Suche & Wissen

- **Websuche** — Tavily, Exa, Zhipu WebSearch, Bocha mit Quellenangaben und Suchquery-Generierung.
- **Lokale Wissensbasis** — Private Dokumente mit sqlite-vec indexieren, Retrieval/Rerank konfigurieren und Feedback prüfen.
- **Kontextverwaltung** — Dateien, Suchergebnisse, Wissensausschnitte, Erinnerungen und Tool-Ausgaben anhängen.

## Werkzeuge & Erweiterungen

- **MCP-Protokoll** — Model Context Protocol-Server über stdio, SSE oder StreamableHTTP ausführen.
- **Integrierte Tools** — @aqbot/fetch und Dateisuche ohne separaten Server nutzen.
- **Tool-Loop-Limit** — Maximale MCP Tool-Call-Schleifen konfigurieren und blockierte Sessions besser wiederherstellen.

## API-Gateway

- **Lokales Gateway** — OpenAI Chat Completions, OpenAI Responses, Claude-native und Gemini-native Endpoints lokal bereitstellen.
- **Zugriff und Beobachtung** — Gateway-Schlüssel, SSL/TLS, Request-Logs und Nutzungsanalysen lokal verwalten.
- **Client-Templates** — Vorlagen für Claude Code, Codex CLI, OpenCode, Gemini CLI und Custom Clients.

## Datenimport & Backup

- **Drittanbieter-Importe** — ChatGPT, Cherry Studio und Kelivo mit Vorschau, Warnungen und Duplikatbehandlung importieren.
- **Provider- und Dateimigration** — Cherry Studio/Kelivo können Provider, API Keys und Anhänge optional migrieren.
- **Backups** — Backup/Restore über lokale Ordner, WebDAV oder S3-kompatiblen Speicher.

## Desktop & Sicherheit

- **Lokale Verschlüsselung** — App-Status in ~/.aqbot/, Benutzerdateien in ~/Documents/aqbot/, API Keys mit AES-256 geschützt.
- **Desktop-Integration** — Tray, Always-on-top, globale Shortcuts, Auto-Start, Proxy und Update-Prüfung.
- **11 UI-Sprachen** — Umschalten zwischen Chinesisch, Englisch, Japanisch, Koreanisch, Französisch, Deutsch, Spanisch, Russisch, Hindi und Arabisch.
