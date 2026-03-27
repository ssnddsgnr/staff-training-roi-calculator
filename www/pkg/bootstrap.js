import init, { calculate } from './pkg/training_roi_calculator.js';

// ─── Инициализация WASM ───────────────────────────────────────────────────────

await init();

// ─── Вспомогательные функции ──────────────────────────────────────────────────

const $ = id => document.getElementById(id);

function fmt(n) {
  return '$' + Math.round(n).toLocaleString('ru-RU');
}

function get(id) {
  return parseFloat($(id).value);
}

// ─── Сбор входных данных со слайдеров ─────────────────────────────────────────

function readInputs() {
  return {
    salary:                  get('salary'),
    replacement_factor:      get('kfactor'),
    onboarding_months:       get('onboard'),
    productivity_loss:       get('prodloss') / 100,
    training_direct_cost:    get('traincost'),
    mentor_hours_per_month:  get('mentor-hours'),
    mentor_months:           get('mentor-months'),
    mentor_hourly_rate:      get('mentor-rate'),
    retention_horizon_years: get('horizon'),
    retention_probability:   get('retention') / 100,
  };
}

// ─── Обновление отображения значений слайдеров ────────────────────────────────

function updateLabels() {
  $('salary-out').textContent     = fmt(get('salary'));
  $('k-out').textContent          = get('kfactor').toFixed(2) + '×';
  $('onboard-out').textContent    = get('onboard') + ' мес.';
  $('prodloss-out').textContent   = get('prodloss') + '%';
  $('train-out').textContent      = fmt(get('traincost'));
  $('mentor-hours-out').textContent = get('mentor-hours') + ' ч.';
  $('mentor-months-out').textContent = get('mentor-months') + ' мес.';
  $('mentor-rate-out').textContent  = '$' + get('mentor-rate') + '/ч';

  const h = get('horizon');
  $('horizon-out').textContent    = h % 1 === 0 ? h + ' ' + plural(h, 'год','года','лет') : h + ' г.';
  $('retention-out').textContent  = get('retention') + '%';
}

function plural(n, one, few, many) {
  const mod10 = n % 10, mod100 = n % 100;
  if (mod10 === 1 && mod100 !== 11) return one;
  if (mod10 >= 2 && mod10 <= 4 && (mod100 < 10 || mod100 >= 20)) return few;
  return many;
}

// ─── Запись результатов в DOM ─────────────────────────────────────────────────

function renderResult(r) {
  $('c-replace').textContent  = fmt(r.cost_replacement);
  $('c-train').textContent    = fmt(r.cost_training_total);
  $('c-mentor').textContent   = fmt(r.cost_mentor);
  $('c-expected').textContent = fmt(r.expected_replacement_cost);
  $('roi-out').textContent    = Math.round(r.roi_percent) + '%';
  $('saving-out').textContent = fmt(r.net_saving);

  $('b-recruit').textContent  = fmt(r.breakdown_recruitment);
  $('b-prod').textContent     = fmt(r.breakdown_productivity_loss);
  $('b-total').textContent    = fmt(r.cost_replacement);

  // ROI цвет
  const roiEl = $('roi-out');
  roiEl.className = 'mval ' + (r.roi_percent >= 0 ? 'green' : 'red');

  const savingEl = $('saving-out');
  savingEl.className = 'mval ' + (r.net_saving >= 0 ? 'green' : 'red');

  // Вердикт
  const v = $('verdict');
  if (r.verdict === 'Train') {
    v.className = 'verdict train';
    v.textContent = 'Обучать выгоднее — экономия ' + fmt(r.net_saving) + ' на одном сотруднике';
  } else if (r.verdict === 'Hire') {
    v.className = 'verdict hire';
    v.textContent = 'В этих условиях дешевле нанять готового специалиста';
  } else {
    v.className = 'verdict neutral';
    v.textContent = 'Разница несущественна (< 5%) — решение зависит от других факторов';
  }
}

// ─── Главный цикл ─────────────────────────────────────────────────────────────

function update() {
  updateLabels();
  try {
    const result = calculate(readInputs());
    renderResult(result);
  } catch (e) {
    console.error('WASM error:', e);
  }
}

// Навешиваем на все слайдеры
const sliders = [
  'salary','kfactor','onboard','prodloss',
  'traincost','mentor-hours','mentor-months','mentor-rate',
  'horizon','retention'
];
sliders.forEach(id => $(id).addEventListener('input', update));

// Первый рендер
update();
