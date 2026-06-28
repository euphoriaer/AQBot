# الميزات

## الدردشة والنماذج

- **Multi-provider chat** — اربط OpenAI وClaude وGemini وDeepSeek وQwen وأي OpenAI-compatible endpoint مع Base URL وAPI Path وheaders وproxy rules.
- **تهيئة المزوّدين** — استخدم aqbot:// provider links وCC Switch import لجلب provider profiles إلى AQBot بعد تأكيد المستخدم.
- **إدارة النماذج** — زامن remote model lists، ونظّم groups، واختبر latency، واضبط capabilities وcontext length وsampling defaults وreasoning profiles وper-model extra_body.
- **مسارات المحادثة** — Streaming replies وthinking blocks وmessage versions وbranches وtitle-generation status وlong chat compression وmulti-model comparison.

## AI Agent

- **Agent mode** — يمكن للنموذج تعديل الملفات وتشغيل الأوامر وتحليل الكود داخل desktop workflow مضبوط.
- **التحكم في الصلاحيات** — اختر standard review أو auto-accept edits أو full-access mode مع استمرار working-directory sandbox checks.
- **الموافقة والتكلفة** — راجع tool calls لحظياً، واحفظ allow decisions، وتابع token/cost لكل session.

## Roles

- **Local role management** — احفظ system prompts وavatars وtags وopening messages وstarter questions وtemperature وTop P كقوالب conversation قابلة لإعادة الاستخدام.
- **One-click use** — السلوك الافتراضي ينشئ role conversation جديداً، ويمكن تطبيق role على المحادثة الحالية من dropdown؛ role chats تحتفظ بالاسم وavatar وblue Roles badge.
- **Online marketplace** — ابحث وثبّت roles من prompts.chat وPlexPt 中文 ثم استخدمها محلياً.

## Skills Management

- **Multi-source skill directories** — أدر AQBot وCodex وClaude وAgents skill roots، بما فيها `~/.aqbot/skills` و`~/.codex/skills` و`~/.claude/skills` و`~/.agents/skills`.
- **My Skills** — يدعم source filter وenable/disable وdetail view وcopy name وopen directory وuninstall.
- **Skill groups and install targets** — اطوِ حسب group، ونفّذ bulk enable/disable، وافتح group folder، واحذف whole group، وثبّت من `owner/repo` أو GitHub URL إلى target تختاره.
- **Marketplace** — ابحث في skills.sh وGitHub، وعاين details، وافتح GitHub، واعرض installed status.

## عرض المحتوى

- **Markdown والمعادلات** — اعرض Markdown وcode highlighting وtables وtask lists وLaTeX formulas داخل streaming conversations.
- **الكود والمخططات وArtifact** — استخدم Monaco code blocks وMermaid وD2 وArtifact panel للكود والملاحظات والتقارير والمعاينات.
- **HTML fragments** — عاين generated HTML fragments بأمان مع أحدث streaming stability fixes.

## البحث والمعرفة

- **Web search** — يدعم Tavily وExa وZhipu WebSearch وBocha مع cited sources وgenerated search queries.
- **Local knowledge base** — افهرس private documents باستخدام sqlite-vec واضبط retrieval/rerank options وراجع retrieval feedback.
- **Context management** — أضف files وsearch results وknowledge snippets وmemories وtool output إلى conversation context.

## الأدوات والامتدادات

- **MCP protocol** — شغّل Model Context Protocol servers عبر stdio أو SSE أو StreamableHTTP.
- **Built-in tools** — استخدم @aqbot/fetch وfile search بدون تثبيت server منفصل.
- **Tool loop limit** — اضبط MCP tool-call loop count واستعد بشكل أفضل من interrupted أو stuck tool sessions.

## API gateway

- **Local gateway** — اعرض OpenAI Chat Completions وOpenAI Responses وClaude-native وGemini-native endpoints من desktop app.
- **الوصول والمراقبة** — أدر gateway keys وSSL/TLS certificates وrequest logs وusage analytics محلياً.
- **Client templates** — Templates جاهزة لـ Claude Code وCodex CLI وOpenCode وGemini CLI وcustom clients.

## استيراد البيانات والنسخ الاحتياطي

- **Third-party imports** — استورد ChatGPT official exports وCherry Studio backups وKelivo backups مع preview counts وwarnings وduplicate handling.
- **Provider and file migration** — يمكن لـ Cherry Studio/Kelivo import نقل linked providers وAPI keys وfile attachments اختيارياً.
- **Backups** — انسخ واستعد البيانات عبر local folders أو WebDAV أو S3-compatible storage.

## سطح المكتب والأمان

- **Local encryption** — يُحفظ app state في ~/.aqbot/ وuser files في ~/Documents/aqbot/، وتُحمى API keys عبر AES-256 وlocal master key.
- **Desktop integration** — Tray وalways-on-top وglobal shortcuts وauto-start وproxy settings وautomatic update checks.
- **11 interface languages** — بدّل بين Simplified Chinese وTraditional Chinese وEnglish وJapanese وKorean وFrench وGerman وSpanish وRussian وHindi وArabic.
