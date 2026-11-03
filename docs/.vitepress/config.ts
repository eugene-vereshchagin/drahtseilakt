import { defineConfig } from "vitepress";

export default defineConfig({
  title: "OpenBallance",
  description: "A faithful reimplementation of Ballance (2004): the original Virtools scripts on a new runtime, using the data from your own CD.",
  cleanUrls: true,
  lastUpdated: true,
  srcExclude: ["README.md", "AGENTS.md"],
  sitemap: { hostname: "https://openballance.emdzej.pl" },
  // The browser player is a static page in public/play/, not a Markdown page.
  ignoreDeadLinks: [/^\/play\//],

  // English is the root locale; every other language mirrors the same paths under /<lang>/ (see CLAUDE.md).
  locales: {
    root: { label: "English", lang: "en" },
    ru: {
      label: "Русский",
      lang: "ru",
      link: "/ru/",
      themeConfig: {
        nav: [
          { text: "План работ", link: "/ru/drahtseilakt/roadmap" },
          { text: "Решения", link: "/ru/drahtseilakt/decisions" },
          { text: "Документация (англ.)", link: "/guide/" },
        ],
        sidebar: {
          "/ru/": [
            {
              text: "Drahtseilakt",
              items: [
                { text: "История", link: "/ru/drahtseilakt/history" },
                { text: "В цифрах", link: "/ru/drahtseilakt/stats" },
                { text: "План работ", link: "/ru/drahtseilakt/roadmap" },
                { text: "Решения", link: "/ru/drahtseilakt/decisions" },
                { text: "Меню", link: "/ru/drahtseilakt/menu" },
                { text: "Тестовый игрок", link: "/ru/drahtseilakt/autopilot" },
                { text: "Как ездит тестовый игрок", link: "/ru/drahtseilakt/autopilot-rules" },
                { text: "Ресурсы интерфейса", link: "/ru/drahtseilakt/resources" },
                { text: "Звуковые темы", link: "/ru/drahtseilakt/sound-themes" },
                { text: "Пакеты текстур и моделей", link: "/ru/drahtseilakt/packs" },
                { text: "Моды", link: "/ru/drahtseilakt/mods" },
                { text: "Где лежат уровни", link: "/ru/drahtseilakt/levels" },
                { text: "Любительские карты", link: "/ru/drahtseilakt/community-maps" },
                { text: "Карты: где они сейчас", link: "/ru/drahtseilakt/community-maps-status" },
                { text: "Формат файлов Virtools", link: "/ru/drahtseilakt/nmo-format" },
                { text: "Открытый формат уровней", link: "/ru/drahtseilakt/open-format" },
                { text: "Фаззинг парсеров", link: "/ru/drahtseilakt/fuzzing" },
              ],
            },
          ],
        },
        outline: { level: [2, 3], label: "На этой странице" },
        docFooter: { prev: "Назад", next: "Далее" },
        lastUpdated: { text: "Обновлено" },
        darkModeSwitchLabel: "Тема",
        sidebarMenuLabel: "Меню",
        returnToTopLabel: "Наверх",
        langMenuLabel: "Язык",
      },
    },
    de: {
      label: "Deutsch",
      lang: "de",
      link: "/de/",
      themeConfig: {
        nav: [
          { text: "Fahrplan", link: "/de/drahtseilakt/roadmap" },
          { text: "Entscheidungen", link: "/de/drahtseilakt/decisions" },
          { text: "Dokumentation (engl.)", link: "/guide/" },
        ],
        sidebar: {
          "/de/": [
            {
              text: "Drahtseilakt",
              items: [
                { text: "Geschichte", link: "/de/drahtseilakt/history" },
                { text: "In Zahlen", link: "/de/drahtseilakt/stats" },
                { text: "Fahrplan", link: "/de/drahtseilakt/roadmap" },
                { text: "Entscheidungen", link: "/de/drahtseilakt/decisions" },
                { text: "Menüs", link: "/de/drahtseilakt/menu" },
                { text: "Testspieler", link: "/de/drahtseilakt/autopilot" },
                { text: "Wie der Testspieler fährt", link: "/de/drahtseilakt/autopilot-rules" },
                { text: "Ressourcen der Oberfläche", link: "/de/drahtseilakt/resources" },
                { text: "Klangthemen", link: "/de/drahtseilakt/sound-themes" },
                { text: "Textur- und Modellpakete", link: "/de/drahtseilakt/packs" },
                { text: "Mods", link: "/de/drahtseilakt/mods" },
                { text: "Wo die Level liegen", link: "/de/drahtseilakt/levels" },
                { text: "Community-Karten", link: "/de/drahtseilakt/community-maps" },
                { text: "Karten: der Stand", link: "/de/drahtseilakt/community-maps-status" },
                { text: "Virtools-Dateiformat", link: "/de/drahtseilakt/nmo-format" },
                { text: "Offenes Levelformat", link: "/de/drahtseilakt/open-format" },
                { text: "Fuzzing der Parser", link: "/de/drahtseilakt/fuzzing" },
              ],
            },
          ],
        },
        outline: { level: [2, 3], label: "Auf dieser Seite" },
        docFooter: { prev: "Zurück", next: "Weiter" },
        lastUpdated: { text: "Aktualisiert" },
        darkModeSwitchLabel: "Darstellung",
        sidebarMenuLabel: "Menü",
        returnToTopLabel: "Nach oben",
        langMenuLabel: "Sprache",
      },
    },
    zh: {
      label: "简体中文",
      lang: "zh-CN",
      link: "/zh/",
      themeConfig: {
        nav: [
          { text: "路线图", link: "/zh/drahtseilakt/roadmap" },
          { text: "决定", link: "/zh/drahtseilakt/decisions" },
          { text: "文档（英文）", link: "/guide/" },
        ],
        sidebar: {
          "/zh/": [
            {
              text: "Drahtseilakt",
              items: [
                { text: "历史", link: "/zh/drahtseilakt/history" },
                { text: "数字", link: "/zh/drahtseilakt/stats" },
                { text: "路线图", link: "/zh/drahtseilakt/roadmap" },
                { text: "决定", link: "/zh/drahtseilakt/decisions" },
                { text: "菜单", link: "/zh/drahtseilakt/menu" },
                { text: "测试玩家", link: "/zh/drahtseilakt/autopilot" },
                { text: "测试玩家的驾驶规则", link: "/zh/drahtseilakt/autopilot-rules" },
                { text: "界面资源", link: "/zh/drahtseilakt/resources" },
                { text: "音效主题", link: "/zh/drahtseilakt/sound-themes" },
                { text: "纹理包与模型包", link: "/zh/drahtseilakt/packs" },
                { text: "模组", link: "/zh/drahtseilakt/mods" },
                { text: "关卡存放位置", link: "/zh/drahtseilakt/levels" },
                { text: "自制地图", link: "/zh/drahtseilakt/community-maps" },
                { text: "自制地图现状", link: "/zh/drahtseilakt/community-maps-status" },
                { text: "Virtools 文件格式", link: "/zh/drahtseilakt/nmo-format" },
                { text: "开放关卡格式", link: "/zh/drahtseilakt/open-format" },
                { text: "解析器模糊测试", link: "/zh/drahtseilakt/fuzzing" },
              ],
            },
          ],
        },
        outline: { level: [2, 3], label: "本页内容" },
        docFooter: { prev: "上一页", next: "下一页" },
        lastUpdated: { text: "更新于" },
        darkModeSwitchLabel: "主题",
        sidebarMenuLabel: "菜单",
        returnToTopLabel: "回到顶部",
        langMenuLabel: "语言",
      },
    },
  },

  head: [
    ["link", { rel: "icon", href: "/favicon.png", type: "image/png" }],
    ["meta", { name: "theme-color", content: "#e8835a" }],
    ["meta", { property: "og:title", content: "OpenBallance: Ballance, rolling again" }],
    ["meta", { property: "og:description", content: "A faithful reimplementation of Ballance: the original game scripts on a new Virtools-compatible runtime with a port of its physics engine. Runs on gasm, natively and in the browser." }],
    ["meta", { property: "og:image", content: "https://openballance.emdzej.pl/screenshots/level01.jpg" }],
    ["meta", { property: "og:url", content: "https://openballance.emdzej.pl/" }],
  ],

  themeConfig: {
    siteTitle: "OpenBallance",
    logo: "/favicon.png",

    nav: [
      { text: "User guide", link: "/guide/", activeMatch: "/guide/" },
      { text: "How-tos", link: "/howto/build-from-source", activeMatch: "/howto/" },
      {
        text: "Internals",
        link: "/internals/",
        activeMatch: "/(internals|ck-runtime|gameplay_bbs|fonts|sky|animation|particles|physics|ivp_)",
      },
      { text: "Gallery", link: "/gallery" },
      // A static app in docs/public/play/: target _self makes it a full page load, not a VitePress route.
      { text: "Play", link: "/play/", target: "_self" },
    ],

    sidebar: {
      "/guide/": [
        {
          text: "User guide",
          items: [
            { text: "Introduction", link: "/guide/" },
            { text: "Status and compatibility", link: "/guide/status" },
            { text: "Installing", link: "/guide/install" },
            { text: "Game data", link: "/guide/game-data" },
            { text: "Running on gasm", link: "/guide/running" },
            { text: "Playing in the browser", link: "/guide/browser" },
            { text: "Controls", link: "/guide/controls" },
            { text: "Launch parameters", link: "/guide/parameters" },
            { text: "Troubleshooting", link: "/guide/troubleshooting" },
          ],
        },
      ],
      "/howto/": [
        {
          text: "How-tos",
          items: [
            { text: "Build from source", link: "/howto/build-from-source" },
            { text: "Run the tests", link: "/howto/tests" },
            { text: "Headless runs and screenshots", link: "/howto/headless" },
            { text: "Make a release", link: "/howto/release" },
            { text: "Reverse-engineering workflow", link: "/howto/reverse-engineering" },
          ],
        },
      ],
      "/": [
        {
          text: "Drahtseilakt",
          items: [
            { text: "History", link: "/drahtseilakt/history" },
            { text: "By the numbers", link: "/drahtseilakt/stats" },
            { text: "Roadmap", link: "/drahtseilakt/roadmap" },
            { text: "Decisions", link: "/drahtseilakt/decisions" },
            { text: "Menus", link: "/drahtseilakt/menu" },
            { text: "Test player", link: "/drahtseilakt/autopilot" },
            { text: "How the test player drives", link: "/drahtseilakt/autopilot-rules" },
            { text: "Interface resources", link: "/drahtseilakt/resources" },
            { text: "Sound themes", link: "/drahtseilakt/sound-themes" },
            { text: "Texture and model packs", link: "/drahtseilakt/packs" },
            { text: "Mods", link: "/drahtseilakt/mods" },
            { text: "Where levels live", link: "/drahtseilakt/levels" },
            { text: "Community maps", link: "/drahtseilakt/community-maps" },
            { text: "Community maps: where they stand", link: "/drahtseilakt/community-maps-status" },
            { text: "Virtools file format", link: "/drahtseilakt/nmo-format" },
            { text: "Open level format", link: "/drahtseilakt/open-format" },
            { text: "Fuzzing the parsers", link: "/drahtseilakt/fuzzing" },
          ],
        },
        {
          text: "Internals",
          items: [
            { text: "Overview", link: "/internals/" },
            { text: "Architecture", link: "/internals/architecture" },
            { text: "Virtools runtime (CK2)", link: "/ck-runtime" },
          ],
        },
        {
          text: "Building Blocks",
          items: [
            { text: "Gameplay BBs", link: "/gameplay_bbs" },
            { text: "Fonts and 2D text", link: "/fonts" },
            { text: "Sky (TT Sky)", link: "/sky" },
            { text: "Curves and animations", link: "/animation" },
            { text: "Particle systems", link: "/particles" },
          ],
        },
        {
          text: "Physics (Ipion)",
          items: [
            { text: "Physics BBs and manager", link: "/physics" },
            { text: "IVP core and dynamics", link: "/ivp_core" },
            { text: "IVP collision detection", link: "/ivp_collision" },
            { text: "IVP contact response", link: "/ivp_contact" },
          ],
        },
        {
          text: "Design records",
          items: [
            { text: "Initial analysis", link: "/internals/analysis" },
            { text: "gasm gfx requirements", link: "/internals/gasm-gfx-requirements" },
          ],
        },
      ],
    },

    socialLinks: [{ icon: "github", link: "https://github.com/emdzej/openballance" }],

    editLink: {
      pattern: "https://github.com/emdzej/openballance/edit/main/docs/:path",
      text: "Edit this page on GitHub",
    },

    outline: { level: [2, 3] },

    search: { provider: "local" },

    footer: {
      message:
        "OpenBallance is released under the GPL-3.0. Ballance is © 2004 Cyparade / Atari. OpenBallance contains no original code or assets: it runs the game from your own copy. It runs on <a href=\"https://gasm.emdzej.pl\">gasm</a>.",
    },
  },
});
