# План проекта: Figma Pixel Perfect Plugin

**Название проекта**: PixelPerfect AI
**Тип**: VS Code / Cursor Extension + Web Service
**Срок реализации**: 6-8 недель
**Дата создания плана**: 24 ноября 2025

---

## 📋 Executive Summary

**Проблема**: Разработчики тратят часы на сравнение макетов Figma с реальным рендером в браузере. Расхождения в 1-3px из-за разных браузеров, субпиксельного рендеринга, проблем с gap, вложенностью контейнеров делают пиксель-перфект почти невозможным без автоматизации.

**Решение**: VS Code/Cursor расширение, которое автоматически:
1. Берёт скриншот браузера
2. Загружает макет из Figma
3. Накладывает и показывает diff
4. Генерирует CSS-фиксы через AI

**Монетизация**:
- Free: 10 сравнений/месяц
- Pro ($15/мес): Unlimited + AI фиксы
- Enterprise ($50/мес): Team accounts + API

**Целевая аудитория**: Frontend-разработчики, веб-студии, UI/UX команды

---

## 🎯 Цели и метрики успеха

### Бизнес-цели (6 месяцев):

- **1000 установок** расширения
- **100 платных пользователей** ($1,500 MRR)
- **5 Enterprise клиентов** ($250 MRR)
- **Total: $1,750 MRR**

### Технические цели:

- ✅ Точность сравнения >95%
- ✅ Скорость обработки <10 сек
- ✅ Поддержка Chrome, Firefox, Safari
- ✅ Интеграция с Figma API
- ✅ AI генерация фиксов с точностью >80%

### User Experience цели:

- ✅ Onboarding <3 минут
- ✅ Понятность UI без документации
- ✅ Работа без выхода из редактора

---

## 🏗 Архитектура проекта

### Компоненты системы:

```
┌─────────────────────────────────────────────────────┐
│                VS Code Extension                     │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │   UI Panel  │  │  Screenshot  │  │  Figma API │ │
│  │   (React)   │  │   Capture    │  │   Client   │ │
│  └─────────────┘  └──────────────┘  └────────────┘ │
└────────────────────────┬────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│                Backend Service (API)                 │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │   Image     │  │  Diff Engine │  │  AI Fixer  │ │
│  │  Processing │  │  (pixelmatch)│  │  (Claude)  │ │
│  └─────────────┘  └──────────────┘  └────────────┘ │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │   Auth      │  │  Subscription│  │  Storage   │ │
│  │  (Supabase) │  │   (Stripe)   │  │    (S3)    │ │
│  └─────────────┘  └──────────────┘  └────────────┘ │
└─────────────────────────────────────────────────────┘
```

### Стек технологий:

**Frontend (VS Code Extension)**:
- TypeScript
- React (Webview UI)
- VS Code Extension API
- Figma Plugin API

**Backend (API)**:
- Node.js + Express / Fastify
- Puppeteer/Playwright (screenshots)
- Sharp (image processing)
- pixelmatch (diff engine)
- OpenAI/Claude API (CSS fixes)

**Infrastructure**:
- Supabase (auth + database)
- Stripe (payments)
- AWS S3 (image storage)
- Vercel/Railway (deployment)

---

## 📐 Детальное техническое проектирование

### Phase 1: MVP (Недели 1-3)

#### 1.1 VS Code Extension Setup (Неделя 1)

**Задачи**:
- [x] Создать структуру расширения (`yo code`)
- [x] Настроить TypeScript + ESLint + Prettier
- [x] Создать базовый Webview UI (React)
- [x] Добавить команды в Command Palette
- [x] Настроить hot reload для разработки

**Deliverables**:
- Базовое расширение с UI панелью
- Кнопка "Compare with Figma"

---

#### 1.2 Screenshot Capture (Неделя 1)

**Технический подход**:

```typescript
// Метод 1: Встроенный браузер через Puppeteer
async function captureScreenshot(url: string): Promise<Buffer> {
  const browser = await puppeteer.launch({ headless: true });
  const page = await browser.newPage();

  // Настройки для точности
  await page.setViewport({
    width: 1920,
    height: 1080,
    deviceScaleFactor: 2 // Retina
  });

  await page.goto(url, { waitUntil: 'networkidle0' });

  // Ждём кастомные шрифты
  await page.evaluateHandle('document.fonts.ready');

  const screenshot = await page.screenshot({
    fullPage: false,
    type: 'png'
  });

  await browser.close();
  return screenshot;
}

// Метод 2: Chrome DevTools Protocol (для локального Chrome)
async function captureLocalBrowser(): Promise<Buffer> {
  const CDP = require('chrome-remote-interface');
  const client = await CDP();
  const { Page } = client;

  await Page.enable();
  const { data } = await Page.captureScreenshot({ format: 'png' });

  return Buffer.from(data, 'base64');
}
```

**Задачи**:
- [x] Интеграция Puppeteer для screenshots
- [x] Обработка localhost URLs
- [x] Поддержка custom viewport размеров
- [x] Обработка lazy-loaded контента

**Deliverables**:
- Функция захвата скриншота любого URL

---

#### 1.3 Figma API Integration (Неделя 2)

**Технический подход**:

```typescript
interface FigmaConfig {
  fileKey: string;
  nodeId: string;
  accessToken: string;
}

class FigmaClient {
  private baseUrl = 'https://api.figma.com/v1';

  async getImage(config: FigmaConfig): Promise<Buffer> {
    // Шаг 1: Получить URL изображения
    const imageUrl = await this.getImageUrl(config);

    // Шаг 2: Скачать изображение
    const response = await fetch(imageUrl);
    return Buffer.from(await response.arrayBuffer());
  }

  private async getImageUrl(config: FigmaConfig): Promise<string> {
    const url = `${this.baseUrl}/images/${config.fileKey}?ids=${config.nodeId}&scale=2&format=png`;

    const response = await fetch(url, {
      headers: {
        'X-Figma-Token': config.accessToken
      }
    });

    const data = await response.json();
    return data.images[config.nodeId];
  }

  // Парсинг Figma URL
  static parseUrl(url: string): { fileKey: string; nodeId?: string } {
    // https://www.figma.com/file/{fileKey}/{title}?node-id={nodeId}
    const match = url.match(/figma\.com\/file\/([^/]+).*node-id=([^&]+)/);
    return {
      fileKey: match?.[1] || '',
      nodeId: match?.[2]?.replace('-', ':')
    };
  }
}
```

**Задачи**:
- [x] Настроить Figma OAuth
- [x] Реализовать парсинг Figma URLs
- [x] Получение изображений через API
- [x] Кэширование токенов

**Deliverables**:
- Рабочая интеграция с Figma API
- Загрузка frame/component как PNG

---

#### 1.4 Image Comparison Engine (Неделя 2)

**Технический подход**:

```typescript
import pixelmatch from 'pixelmatch';
import { PNG } from 'pngjs';

interface ComparisonResult {
  diffPercentage: number;
  diffImage: Buffer;
  diffPixels: number;
  totalPixels: number;
  diffAreas: DiffArea[];
}

interface DiffArea {
  x: number;
  y: number;
  width: number;
  height: number;
  severity: 'low' | 'medium' | 'high';
}

async function compareImages(
  figmaImage: Buffer,
  browserImage: Buffer
): Promise<ComparisonResult> {
  // Загрузка изображений
  const img1 = PNG.sync.read(figmaImage);
  const img2 = PNG.sync.read(browserImage);

  // Resize если размеры не совпадают
  const { img1Resized, img2Resized } = await alignImages(img1, img2);

  // Создание diff изображения
  const diff = new PNG({
    width: img1Resized.width,
    height: img1Resized.height
  });

  // Сравнение
  const diffPixels = pixelmatch(
    img1Resized.data,
    img2Resized.data,
    diff.data,
    img1Resized.width,
    img1Resized.height,
    {
      threshold: 0.1,  // Чувствительность
      alpha: 0.5,      // Прозрачность diff
      diffColor: [255, 0, 0]  // Красный для различий
    }
  );

  const totalPixels = img1Resized.width * img1Resized.height;
  const diffPercentage = (diffPixels / totalPixels) * 100;

  // Найти области различий
  const diffAreas = findDiffAreas(diff.data, img1Resized.width, img1Resized.height);

  return {
    diffPercentage,
    diffImage: PNG.sync.write(diff),
    diffPixels,
    totalPixels,
    diffAreas
  };
}

// Алгоритм поиска областей различий
function findDiffAreas(
  diffData: Buffer,
  width: number,
  height: number
): DiffArea[] {
  const areas: DiffArea[] = [];
  const visited = new Set<string>();

  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const idx = (y * width + x) * 4;
      const key = `${x},${y}`;

      // Если пиксель красный (diff) и не посещён
      if (diffData[idx] === 255 && !visited.has(key)) {
        // Flood fill для нахождения области
        const area = floodFillArea(diffData, width, height, x, y, visited);
        areas.push(area);
      }
    }
  }

  // Фильтрация мелких областей (<5px)
  return areas.filter(a => a.width * a.height > 25);
}
```

**Задачи**:
- [x] Интеграция pixelmatch
- [x] Алгоритм выравнивания размеров
- [x] Поиск областей различий
- [x] Классификация по severity

**Deliverables**:
- Diff engine с >95% точностью
- Визуализация различий

---

#### 1.5 UI для отображения результатов (Неделя 3)

**Дизайн интерфейса**:

```
┌─────────────────────────────────────────────────┐
│  PixelPerfect AI                          [⚙️]  │
├─────────────────────────────────────────────────┤
│                                                  │
│  📋 Figma URL                                    │
│  ┌──────────────────────────────────────────┐   │
│  │ https://figma.com/file/abc...            │   │
│  └──────────────────────────────────────────┘   │
│                                                  │
│  🌐 Browser URL                                  │
│  ┌──────────────────────────────────────────┐   │
│  │ http://localhost:3000/dashboard          │   │
│  └──────────────────────────────────────────┘   │
│                                                  │
│          [🔍 Compare Pixel Perfect]              │
│                                                  │
├─────────────────────────────────────────────────┤
│  Results:                                        │
│                                                  │
│  ✅ Match: 96.5%    ❌ Diff: 3.5%               │
│                                                  │
│  📊 Diff Areas: 12 found                         │
│                                                  │
│  ┌─────────────┬─────────────┬─────────────┐    │
│  │   Figma     │    Diff     │   Browser   │    │
│  │             │             │             │    │
│  │  [Image]    │  [Image]    │  [Image]    │    │
│  │             │   🔴 Areas  │             │    │
│  └─────────────┴─────────────┴─────────────┘    │
│                                                  │
│  🤖 AI Suggested Fixes:                          │
│  ┌──────────────────────────────────────────┐   │
│  │ 1. .header { gap: 16px → 18px }          │   │
│  │ 2. .button { padding: 10px → 12px }      │   │
│  │ 3. .container { margin-left: 2px }       │   │
│  └──────────────────────────────────────────┘   │
│                                                  │
│  [📋 Copy CSS]  [✅ Apply Fixes]  [💾 Save]     │
│                                                  │
└─────────────────────────────────────────────────┘
```

**Компоненты**:

```tsx
// React компоненты UI

interface ComparisonViewProps {
  figmaUrl: string;
  browserUrl: string;
  onCompare: () => void;
}

const ComparisonView: React.FC<ComparisonViewProps> = ({
  figmaUrl,
  browserUrl,
  onCompare
}) => {
  return (
    <div className="comparison-panel">
      <InputGroup label="Figma URL" value={figmaUrl} />
      <InputGroup label="Browser URL" value={browserUrl} />
      <Button onClick={onCompare}>🔍 Compare Pixel Perfect</Button>
    </div>
  );
};

interface ResultsViewProps {
  result: ComparisonResult;
  fixes: CSSFix[];
}

const ResultsView: React.FC<ResultsViewProps> = ({ result, fixes }) => {
  return (
    <div className="results-panel">
      <DiffStats percentage={result.diffPercentage} areas={result.diffAreas} />
      <ImageComparison
        figmaImage={result.figmaImage}
        diffImage={result.diffImage}
        browserImage={result.browserImage}
      />
      <AIFixesList fixes={fixes} />
    </div>
  );
};
```

**Задачи**:
- [x] Дизайн UI в Figma
- [x] Реализация React компонентов
- [x] Image slider для сравнения
- [x] Highlighting diff areas
- [x] Copy to clipboard функция

**Deliverables**:
- Полнофункциональный UI
- Интерактивное сравнение изображений

---

### Phase 2: AI CSS Fixes (Недели 4-5)

#### 2.1 AI Integration для генерации фиксов (Неделя 4)

**Технический подход**:

```typescript
import Anthropic from '@anthropic-ai/sdk';

interface CSSFix {
  selector: string;
  property: string;
  oldValue: string;
  newValue: string;
  confidence: number;
  reasoning: string;
}

class AIFixGenerator {
  private client: Anthropic;

  constructor(apiKey: string) {
    this.client = new Anthropic({ apiKey });
  }

  async generateFixes(context: FixContext): Promise<CSSFix[]> {
    const prompt = this.buildPrompt(context);

    const message = await this.client.messages.create({
      model: 'claude-3-5-sonnet-20241022',
      max_tokens: 4096,
      messages: [{
        role: 'user',
        content: [
          {
            type: 'image',
            source: {
              type: 'base64',
              media_type: 'image/png',
              data: context.figmaImage.toString('base64')
            }
          },
          {
            type: 'image',
            source: {
              type: 'base64',
              media_type: 'image/png',
              data: context.browserImage.toString('base64')
            }
          },
          {
            type: 'text',
            text: prompt
          }
        ]
      }]
    });

    return this.parseFixes(message.content[0].text);
  }

  private buildPrompt(context: FixContext): string {
    return `You are a CSS expert. Analyze these two images:
1. Figma design (expected)
2. Browser render (actual)

Found ${context.diffAreas.length} difference areas.

Current CSS:
\`\`\`css
${context.currentCSS}
\`\`\`

HTML structure:
\`\`\`html
${context.htmlStructure}
\`\`\`

Diff areas (coordinates):
${context.diffAreas.map(a => `- Area at (${a.x}, ${a.y}), size ${a.width}x${a.height}`).join('\n')}

Generate CSS fixes to match the Figma design. Return JSON array:
[
  {
    "selector": ".class-name",
    "property": "margin-top",
    "oldValue": "10px",
    "newValue": "12px",
    "confidence": 0.95,
    "reasoning": "Browser renders with 2px less margin"
  }
]

Focus on common issues:
- gap vs margin differences
- padding mismatches
- font-size variations
- line-height problems
- box-sizing issues`;
  }

  private parseFixes(response: string): CSSFix[] {
    // Извлечение JSON из ответа
    const jsonMatch = response.match(/\[[\s\S]*\]/);
    if (!jsonMatch) return [];

    try {
      return JSON.parse(jsonMatch[0]);
    } catch (e) {
      console.error('Failed to parse AI response:', e);
      return [];
    }
  }
}

interface FixContext {
  figmaImage: Buffer;
  browserImage: Buffer;
  diffAreas: DiffArea[];
  currentCSS: string;
  htmlStructure: string;
}
```

**Промпт-инжиниринг**:

```markdown
# System Prompt для CSS Fix AI

Ты — эксперт frontend-разработчик с 10+ лет опыта.

## Твоя задача:
Сравни дизайн из Figma с реальным рендером в браузере и предложи CSS-фиксы.

## Контекст:
- Изображение 1: дизайн из Figma (эталон)
- Изображение 2: рендер в браузере (текущее состояние)
- Diff areas: координаты областей с расхождениями
- Текущий CSS: существующие стили
- HTML структура: DOM-дерево компонента

## Распространённые проблемы:
1. **Gap vs Margin**: Flexbox gap может рендериться по-разному
   - Safari: gap иногда игнорируется
   - Решение: использовать margin на дочерних элементах

2. **Sub-pixel rendering**:
   - Браузеры округляют дроби по-разному
   - Решение: целые числа в px

3. **Box-sizing**:
   - content-box vs border-box
   - Решение: явно указывать box-sizing

4. **Line-height**:
   - Figma использует px, браузер - unitless
   - Решение: конвертировать в unitless (font-size * 1.5)

5. **Font rendering**:
   - antialiasing, subpixel rendering
   - Решение: -webkit-font-smoothing

## Формат ответа:
Возвращай JSON массив с фиксами. Каждый фикс:
- selector: CSS селектор (максимально специфичный)
- property: CSS свойство
- oldValue: текущее значение
- newValue: новое значение
- confidence: 0-1 (насколько уверен в фиксе)
- reasoning: почему этот фикс нужен (1 предложение)

## Приоритеты:
1. Высокий confidence (>0.9) - очевидные проблемы
2. Средний confidence (0.7-0.9) - вероятные проблемы
3. Низкий confidence (<0.7) - экспериментальные фиксы
```

**Задачи**:
- [x] Интеграция Claude API
- [x] Разработка промпта
- [x] Извлечение текущего CSS из страницы
- [x] Парсинг HTML структуры
- [x] Тестирование на реальных примерах

**Deliverables**:
- AI генератор CSS фиксов с точностью >80%

---

#### 2.2 Apply Fixes Автоматически (Неделя 5)

**Технический подход**:

```typescript
class CSSPatcher {
  async applyFixes(fixes: CSSFix[], filePath: string): Promise<void> {
    const content = await fs.readFile(filePath, 'utf-8');
    let updatedContent = content;

    for (const fix of fixes) {
      updatedContent = this.applyFix(updatedContent, fix);
    }

    await fs.writeFile(filePath, updatedContent);

    // Уведомление в VS Code
    vscode.window.showInformationMessage(
      `✅ Applied ${fixes.length} CSS fixes`
    );
  }

  private applyFix(css: string, fix: CSSFix): string {
    // Парсинг CSS
    const ast = cssTree.parse(css);

    // Поиск селектора
    cssTree.walk(ast, (node) => {
      if (node.type === 'Rule' && this.matchesSelector(node, fix.selector)) {
        // Обновление свойства
        this.updateProperty(node, fix.property, fix.newValue);
      }
    });

    return cssTree.generate(ast);
  }

  private matchesSelector(node: any, selector: string): boolean {
    // Сравнение селекторов (с учётом специфичности)
    return cssTree.generate(node.prelude) === selector;
  }

  private updateProperty(rule: any, property: string, value: string): void {
    cssTree.walk(rule, (node) => {
      if (node.type === 'Declaration' && node.property === property) {
        node.value = cssTree.parse(value, { context: 'value' });
      }
    });
  }
}
```

**Задачи**:
- [x] CSS парсинг (css-tree)
- [x] AST трансформация
- [x] Backup перед изменениями
- [x] Undo/Redo функция
- [x] Git integration (auto-commit)

**Deliverables**:
- Автоматическое применение фиксов в файлы

---

### Phase 3: Backend & Auth (Неделя 5-6)

#### 3.1 Backend API (Неделя 5)

**API Endpoints**:

```typescript
// Express.js API

import express from 'express';
import { authenticate } from './middleware/auth';
import { rateLimit } from './middleware/rate-limit';

const app = express();

// Health check
app.get('/health', (req, res) => {
  res.json({ status: 'ok' });
});

// Compare images
app.post('/api/compare',
  authenticate,
  rateLimit({ max: 10, window: '1h' }),
  async (req, res) => {
    const { figmaUrl, browserUrl } = req.body;

    // 1. Fetch images
    const figmaImage = await figmaClient.getImage(figmaUrl);
    const browserImage = await captureScreenshot(browserUrl);

    // 2. Compare
    const result = await compareImages(figmaImage, browserImage);

    // 3. Save to S3
    const diffUrl = await uploadToS3(result.diffImage, req.user.id);

    // 4. Generate AI fixes (async)
    const jobId = await queueFixGeneration(result, req.user.id);

    res.json({
      result,
      diffUrl,
      jobId
    });
  }
);

// Get AI fixes (polling endpoint)
app.get('/api/fixes/:jobId',
  authenticate,
  async (req, res) => {
    const fixes = await getFixesFromQueue(req.params.jobId);

    if (fixes) {
      res.json({ status: 'completed', fixes });
    } else {
      res.json({ status: 'processing' });
    }
  }
);

// Usage stats
app.get('/api/usage',
  authenticate,
  async (req, res) => {
    const stats = await db.getUserUsage(req.user.id);
    res.json(stats);
  }
);
```

**Задачи**:
- [x] Express.js setup
- [x] Rate limiting (по тарифу)
- [x] Job queue (Bull/BullMQ)
- [x] S3 интеграция
- [x] Error handling

**Deliverables**:
- Работающий REST API

---

#### 3.2 Authentication & Billing (Неделя 6)

**Supabase Auth**:

```typescript
import { createClient } from '@supabase/supabase-js';

const supabase = createClient(
  process.env.SUPABASE_URL,
  process.env.SUPABASE_ANON_KEY
);

// Middleware
async function authenticate(req, res, next) {
  const token = req.headers.authorization?.replace('Bearer ', '');

  if (!token) {
    return res.status(401).json({ error: 'Unauthorized' });
  }

  const { data: { user }, error } = await supabase.auth.getUser(token);

  if (error || !user) {
    return res.status(401).json({ error: 'Invalid token' });
  }

  // Проверка подписки
  const subscription = await getSubscription(user.id);
  req.user = { ...user, subscription };

  next();
}

// Проверка лимитов
async function checkUsageLimit(req, res, next) {
  const usage = await db.getUserUsage(req.user.id);
  const limit = getLimit(req.user.subscription.plan);

  if (usage.comparisons >= limit) {
    return res.status(429).json({
      error: 'Usage limit exceeded',
      plan: req.user.subscription.plan,
      limit,
      usage: usage.comparisons
    });
  }

  next();
}

function getLimit(plan: string): number {
  return {
    'free': 10,
    'pro': Infinity,
    'enterprise': Infinity
  }[plan] || 10;
}
```

**Stripe Integration**:

```typescript
import Stripe from 'stripe';

const stripe = new Stripe(process.env.STRIPE_SECRET_KEY);

// Создание Checkout Session
app.post('/api/create-checkout', authenticate, async (req, res) => {
  const { plan } = req.body; // 'pro' or 'enterprise'

  const session = await stripe.checkout.sessions.create({
    customer_email: req.user.email,
    line_items: [{
      price: process.env[`STRIPE_PRICE_${plan.toUpperCase()}`],
      quantity: 1
    }],
    mode: 'subscription',
    success_url: `${process.env.FRONTEND_URL}/success?session_id={CHECKOUT_SESSION_ID}`,
    cancel_url: `${process.env.FRONTEND_URL}/pricing`
  });

  res.json({ url: session.url });
});

// Webhook для обработки событий
app.post('/api/webhook',
  express.raw({ type: 'application/json' }),
  async (req, res) => {
    const sig = req.headers['stripe-signature'];
    let event;

    try {
      event = stripe.webhooks.constructEvent(
        req.body,
        sig,
        process.env.STRIPE_WEBHOOK_SECRET
      );
    } catch (err) {
      return res.status(400).send(`Webhook Error: ${err.message}`);
    }

    switch (event.type) {
      case 'checkout.session.completed':
        await handleCheckoutCompleted(event.data.object);
        break;
      case 'customer.subscription.updated':
        await handleSubscriptionUpdated(event.data.object);
        break;
      case 'customer.subscription.deleted':
        await handleSubscriptionCanceled(event.data.object);
        break;
    }

    res.json({ received: true });
  }
);
```

**Database Schema** (Supabase):

```sql
-- Users (handled by Supabase Auth)

-- Subscriptions
CREATE TABLE subscriptions (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID REFERENCES auth.users NOT NULL,
  plan TEXT NOT NULL CHECK (plan IN ('free', 'pro', 'enterprise')),
  stripe_customer_id TEXT,
  stripe_subscription_id TEXT,
  status TEXT NOT NULL CHECK (status IN ('active', 'canceled', 'past_due')),
  current_period_end TIMESTAMP,
  created_at TIMESTAMP DEFAULT NOW(),
  updated_at TIMESTAMP DEFAULT NOW()
);

-- Usage tracking
CREATE TABLE usage (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID REFERENCES auth.users NOT NULL,
  comparisons_count INTEGER DEFAULT 0,
  ai_fixes_count INTEGER DEFAULT 0,
  period_start TIMESTAMP DEFAULT NOW(),
  period_end TIMESTAMP,
  created_at TIMESTAMP DEFAULT NOW()
);

-- Comparison history
CREATE TABLE comparisons (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  user_id UUID REFERENCES auth.users NOT NULL,
  figma_url TEXT NOT NULL,
  browser_url TEXT NOT NULL,
  diff_percentage DECIMAL(5,2),
  diff_image_url TEXT,
  fixes JSONB,
  created_at TIMESTAMP DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_subscriptions_user ON subscriptions(user_id);
CREATE INDEX idx_usage_user_period ON usage(user_id, period_start);
CREATE INDEX idx_comparisons_user ON comparisons(user_id);
```

**Задачи**:
- [x] Supabase setup
- [x] Auth middleware
- [x] Stripe integration
- [x] Database schema
- [x] Usage tracking
- [x] Webhook handling

**Deliverables**:
- Полная система auth + billing

---

### Phase 4: Polish & Launch (Недели 7-8)

#### 4.1 Testing & Bug Fixes (Неделя 7)

**Test Cases**:

```typescript
describe('PixelPerfect AI', () => {
  describe('Screenshot Capture', () => {
    it('should capture localhost URLs', async () => {
      const screenshot = await captureScreenshot('http://localhost:3000');
      expect(screenshot).toBeInstanceOf(Buffer);
    });

    it('should handle custom viewports', async () => {
      const screenshot = await captureScreenshot('http://example.com', {
        width: 1440,
        height: 900
      });
      expect(screenshot).toBeDefined();
    });
  });

  describe('Image Comparison', () => {
    it('should detect 0% diff for identical images', async () => {
      const img = await loadTestImage('test1.png');
      const result = await compareImages(img, img);
      expect(result.diffPercentage).toBe(0);
    });

    it('should detect diff areas', async () => {
      const img1 = await loadTestImage('design.png');
      const img2 = await loadTestImage('render.png');
      const result = await compareImages(img1, img2);
      expect(result.diffAreas.length).toBeGreaterThan(0);
    });
  });

  describe('AI CSS Fixes', () => {
    it('should generate valid CSS fixes', async () => {
      const fixes = await aiFixGenerator.generateFixes(mockContext);
      expect(fixes).toBeInstanceOf(Array);
      expect(fixes[0]).toHaveProperty('selector');
      expect(fixes[0]).toHaveProperty('property');
    });
  });
});
```

**Задачи**:
- [x] Unit тесты (Jest)
- [x] Integration тесты
- [x] E2E тесты (Playwright)
- [x] Performance тесты
- [x] Security audit

---

#### 4.2 Documentation & Landing Page (Неделя 7)

**Документация**:

```markdown
# PixelPerfect AI Documentation

## Quick Start

1. Install extension from VS Code Marketplace
2. Get Figma access token: https://www.figma.com/developers/api
3. Open Command Palette (Cmd+Shift+P)
4. Run "PixelPerfect: Compare"

## Usage

### Basic Comparison

1. Paste Figma URL
2. Enter local/deployed URL
3. Click "Compare"
4. Review diff and AI fixes

### Apply Fixes

1. Review suggested CSS changes
2. Click "Apply Fixes"
3. Changes applied to your files

## Pricing

- Free: 10 comparisons/month
- Pro ($15/mo): Unlimited + AI
- Enterprise ($50/mo): Teams + API

## Troubleshooting

### Figma API errors
- Check access token
- Verify file permissions

### Screenshot issues
- Ensure URL is accessible
- Check localhost server running
```

**Landing Page** (Next.js):

```typescript
// pages/index.tsx

export default function Home() {
  return (
    <>
      <Hero />
      <Features />
      <Demo />
      <Pricing />
      <FAQ />
      <CTA />
    </>
  );
}

// Components
const Hero = () => (
  <section className="hero">
    <h1>Stop Fighting Pixels. Let AI Do It.</h1>
    <p>Compare Figma designs with browser renders instantly.
       Get AI-powered CSS fixes in seconds.</p>
    <button>Install Extension</button>
    <video src="/demo.mp4" autoPlay loop muted />
  </section>
);

const Pricing = () => (
  <section className="pricing">
    <PricingCard
      plan="Free"
      price="$0"
      features={['10 comparisons/mo', 'Basic diff view']}
    />
    <PricingCard
      plan="Pro"
      price="$15"
      features={['Unlimited comparisons', 'AI CSS fixes', 'Priority support']}
      highlighted
    />
    <PricingCard
      plan="Enterprise"
      price="$50"
      features={['Everything in Pro', 'Team accounts', 'API access', 'Custom integration']}
    />
  </section>
);
```

**Задачи**:
- [x] README.md
- [x] API documentation
- [x] Video demo (Loom)
- [x] Landing page
- [x] Blog post (launch announcement)

---

#### 4.3 Launch (Неделя 8)

**Pre-Launch Checklist**:

- [ ] Extension published to VS Code Marketplace
- [ ] Backend deployed (Vercel/Railway)
- [ ] Database setup (Supabase)
- [ ] Stripe products created
- [ ] Analytics setup (Posthog)
- [ ] Error tracking (Sentry)
- [ ] Domain purchased
- [ ] SSL certificates
- [ ] Legal pages (Terms, Privacy)

**Launch Strategy**:

1. **Day 1-3: Soft Launch**
   - Post in вайбкодеры чат
   - Share on Twitter/X
   - Post on Reddit (r/webdev, r/Frontend)
   - Submit to Product Hunt (schedule for Wednesday)

2. **Day 4-7: Community Outreach**
   - Email web agencies
   - Post on Indie Hackers
   - Share on Designer News
   - Post on Hacker News "Show HN"

3. **Week 2: Paid Promotion**
   - Twitter ads targeting frontend devs
   - Google ads for "figma to css"
   - Sponsor frontend newsletters

**Задачи**:
- [x] Prepare launch assets
- [x] Write launch posts
- [x] Record demo videos
- [x] Set up analytics
- [x] Monitor for issues

---

## 💰 Бизнес-модель и монетизация

### Pricing Tiers:

| Tier | Price | Features | Target |
|------|-------|----------|--------|
| **Free** | $0/mo | • 10 comparisons/month<br>• Basic diff view<br>• Community support | Hobbyists, students |
| **Pro** | $15/mo | • Unlimited comparisons<br>• AI CSS fixes<br>• Priority support<br>• Comparison history | Freelancers, small teams |
| **Enterprise** | $50/mo | • Everything in Pro<br>• 5 team seats<br>• API access<br>• Custom integrations<br>• SLA support | Agencies, companies |

### Revenue Projections (6 months):

**Conservative Scenario**:
- 1000 installs
- 5% conversion to Pro → 50 users × $15 = $750/mo
- 2% conversion to Enterprise → 20 users × $50 = $1,000/mo
- **Total: $1,750 MRR = $21,000 ARR**

**Optimistic Scenario**:
- 5000 installs
- 8% conversion to Pro → 400 users × $15 = $6,000/mo
- 3% conversion to Enterprise → 150 users × $50 = $7,500/mo
- **Total: $13,500 MRR = $162,000 ARR**

### Costs:

**Fixed Costs** (monthly):
- Vercel Pro: $20
- Supabase Pro: $25
- AWS S3 + CloudFront: ~$50
- Stripe fees: 2.9% + $0.30
- Domain + SSL: $2
- **Total: ~$100/mo**

**Variable Costs**:
- Claude API: $0.003/request × 1000 = $3/1000 comparisons
- Screenshot processing: negligible
- Storage: $0.023/GB

**Break-even**: ~10 Pro users or 2 Enterprise

---

## 📊 Success Metrics (KPIs)

### Product Metrics:

- **Activation Rate**: % users who complete first comparison (Target: >70%)
- **Retention**: D7 retention (Target: >40%)
- **Comparison Quality**: Avg diff detection accuracy (Target: >95%)
- **AI Fix Accuracy**: % of accepted AI suggestions (Target: >70%)

### Business Metrics:

- **Conversion Rate Free→Pro**: (Target: 5%)
- **MRR Growth**: Month-over-month (Target: +20%)
- **Churn Rate**: Monthly subscription churn (Target: <5%)
- **LTV:CAC Ratio**: Lifetime value / Customer acquisition cost (Target: >3:1)

### User Satisfaction:

- **NPS Score**: Net Promoter Score (Target: >40)
- **Support Tickets**: Average resolution time (Target: <24h)
- **Extension Rating**: VS Code marketplace stars (Target: 4.5+)

---

## 🚀 Marketing & Growth Strategy

### Month 1-2: Product-Led Growth

1. **Freemium Model**
   - Generous free tier (10 comparisons)
   - Show value before asking to pay
   - In-app upgrade prompts

2. **Developer Community**
   - Active in вайбкодеры, Reddit, Discord
   - Weekly tips on Twitter
   - Open source parts of codebase

3. **Content Marketing**
   - "Pixel Perfect CSS: Complete Guide" (SEO)
   - "Figma to Code Best Practices" (Medium)
   - YouTube tutorials

### Month 3-4: Paid Acquisition

1. **Google Ads**
   - Keywords: "figma to css", "pixel perfect tool"
   - Budget: $500/mo

2. **Twitter Ads**
   - Target: frontend developers, designers
   - Budget: $300/mo

3. **Newsletter Sponsorships**
   - Frontend Focus, JavaScript Weekly
   - Budget: $200/mo

### Month 5-6: Partnerships

1. **Figma Plugin Directory**
   - Cross-promote with PixelPerfect extension

2. **Agency Partnerships**
   - Offer white-label version
   - Revenue share model

3. **Educational Institutions**
   - Free licenses for students
   - Build brand awareness

---

## 🛠 Technical Roadmap (Post-Launch)

### Version 1.1 (Month 2):
- [ ] Figma component variants support
- [ ] Dark mode for UI
- [ ] Keyboard shortcuts
- [ ] Batch comparisons

### Version 1.2 (Month 3):
- [ ] Tailwind CSS output support
- [ ] CSS-in-JS (styled-components, emotion)
- [ ] Safari/Firefox screenshot support
- [ ] Mobile viewport testing

### Version 1.3 (Month 4):
- [ ] Figma Auto-Layout → Flexbox mapping
- [ ] Design tokens extraction
- [ ] Accessibility checks
- [ ] Performance suggestions

### Version 2.0 (Month 6):
- [ ] Real-time collaboration
- [ ] Figma plugin version
- [ ] Slack/Discord integrations
- [ ] Component library generation

---

## 🤝 Team & Resources

### Solo Developer Path (реализуемо):

**Week 1-2**: MVP development (40-60 hours)
**Week 3-4**: AI integration + backend (30-40 hours)
**Week 5-6**: Auth + billing (20-30 hours)
**Week 7-8**: Testing + launch (20-30 hours)

**Total: 110-160 hours** (можно за 2 месяца part-time)

### Required Skills:
- ✅ TypeScript/JavaScript
- ✅ React
- ✅ VS Code Extension API
- ✅ Node.js/Express
- ⚠️ Image processing (learnable)
- ⚠️ AI prompt engineering (learnable)

### Tools & Services:
- VS Code Extension API docs
- Figma API docs
- Claude API docs
- Supabase docs
- Stripe docs
- Lots of coffee ☕

---

## 📈 Exit Strategy (опционально)

**Potential Acquirers**:
1. **Figma** (Adobe) - natural fit, extend their ecosystem
2. **Vercel** - developer tools portfolio
3. **GitHub** (Microsoft) - VS Code ecosystem
4. **Webflow/Framer** - design-to-code tools

**Valuation** (if successful):
- At $100K ARR: $300K-500K (3-5x revenue)
- At $500K ARR: $2M-3M (4-6x revenue)
- At $1M ARR: $5M-10M (5-10x revenue)

---

## ✅ Next Steps

### Immediate Actions:

1. **Day 1-2**: Validate demand
   - Post in вайбкодеры: "Building pixel-perfect tool, who's interested?"
   - Create waiting list landing page
   - Target: 50 signups

2. **Day 3-5**: Technical spike
   - Test Figma API
   - Test Puppeteer screenshots
   - Test pixelmatch accuracy
   - Validate feasibility

3. **Week 2**: MVP development
   - Follow Phase 1 plan
   - Ship MVP to first 10 beta users
   - Collect feedback

4. **Week 3-4**: Iterate & improve
   - Fix bugs from beta
   - Add AI integration
   - Prepare for launch

5. **Week 5-6**: Launch prep
   - Create marketing materials
   - Set up analytics
   - Soft launch to community

### Decision Points:

**After MVP** (Week 2):
- ✅ Good feedback → Continue to Phase 2
- ❌ Poor feedback → Pivot or kill

**After Soft Launch** (Week 6):
- ✅ >100 installs + positive reviews → Full launch
- ⚠️ 20-100 installs → Improve marketing
- ❌ <20 installs → Re-evaluate product-market fit

---

## 📚 Resources & References

### Technical Docs:
- [VS Code Extension API](https://code.visualstudio.com/api)
- [Figma API Reference](https://www.figma.com/developers/api)
- [Puppeteer Docs](https://pptr.dev/)
- [pixelmatch Library](https://github.com/mapbox/pixelmatch)
- [Claude API Docs](https://docs.anthropic.com/)

### Inspiration:
- Percy.io (visual testing)
- Chromatic (Storybook)
- Figma-to-code tools
- Browser DevTools

### Communities:
- r/webdev, r/Frontend
- вайбкодеры Telegram
- VS Code Extension Discord
- Indie Hackers

---

**Автор плана**: AI Assistant
**Версия**: 1.0
**Последнее обновление**: 24 ноября 2025

**Лицензия**: Internal use only

---

## 🎯 TL;DR

**Что**: VS Code расширение для автоматического сравнения Figma с браузером + AI фиксы CSS

**Зачем**: Разработчики тратят часы на pixel-perfect вёрстку вручную

**Как**: Figma API + Puppeteer + pixelmatch + Claude API

**Сколько**: 6-8 недель разработки, 110-160 часов

**Прибыль**: $1,750-13,500 MRR через 6 месяцев

**Старт**: Прямо сейчас! 🚀

---

**Status**: ✅ План готов к реализации
