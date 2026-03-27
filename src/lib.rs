use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

// ─── Входные параметры ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CalcInput {
    /// Годовая зарплата сотрудника ($)
    pub salary: f64,

    /// Коэффициент замены (рекрутинг + агентство, от годовой зарплаты)
    /// Типичные значения: 0.5 (линейный) → 2.0 (топ-менеджмент)
    pub replacement_factor: f64,

    /// Месяцев до полной продуктивности нового сотрудника
    pub onboarding_months: f64,

    /// Потери продуктивности в период онбординга (0.0 – 1.0)
    pub productivity_loss: f64,

    /// Прямая стоимость обучения ($): курсы, тренинги, материалы
    pub training_direct_cost: f64,

    /// Время ментора в месяц (часов)
    pub mentor_hours_per_month: f64,

    /// Длительность менторства (месяцев)
    pub mentor_months: f64,

    /// Почасовая ставка ментора ($)
    pub mentor_hourly_rate: f64,

    /// Временной горизонт удержания (лет) — на сколько лет считаем ROI
    pub retention_horizon_years: f64,

    /// Вероятность удержания обученного сотрудника за горизонт (0.0 – 1.0)
    pub retention_probability: f64,
}

// ─── Результат ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct CalcResult {
    /// Стоимость замены (рекрутинг + потери продуктивности)
    pub cost_replacement: f64,

    /// Полная стоимость обучения (прямые + время ментора)
    pub cost_training_total: f64,

    /// Из них: прямые расходы
    pub cost_training_direct: f64,

    /// Из них: стоимость времени ментора
    pub cost_mentor: f64,

    /// Ожидаемая стоимость замены с учётом горизонта и вероятности удержания
    /// E[C_replace] = C_replace × (1 − retention_probability)
    /// Если сотрудник уйдёт через horizon лет — заменять придётся снова
    pub expected_replacement_cost: f64,

    /// ROI обучения с учётом горизонта:
    /// ROI = (E[C_replace] − C_train) / C_train × 100%
    pub roi_percent: f64,

    /// Экономия / перерасход в абсолютных числах
    pub net_saving: f64,

    /// Вердикт
    pub verdict: Verdict,

    /// Разбивка стоимости замены
    pub breakdown_recruitment: f64,
    pub breakdown_productivity_loss: f64,
}

#[derive(Serialize)]
pub enum Verdict {
    Train,   // обучать выгоднее
    Hire,    // нанять дешевле
    Neutral, // разница < 5% — статистически несущественно
}

// ─── Основная функция расчёта ─────────────────────────────────────────────────

#[wasm_bindgen]
pub fn calculate(val: JsValue) -> Result<JsValue, JsValue> {
    let input: CalcInput = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&format!("Ошибка входных данных: {e}")))?;

    let result = compute(&input);

    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&format!("Ошибка сериализации: {e}")))
}

fn compute(i: &CalcInput) -> CalcResult {
    let monthly_salary = i.salary / 12.0;

    // ── Стоимость замены ──────────────────────────────────────────────────────
    let recruitment          = i.salary * i.replacement_factor;
    let productivity_loss    = i.onboarding_months * monthly_salary * i.productivity_loss;
    let cost_replacement     = recruitment + productivity_loss;

    // ── Стоимость обучения ────────────────────────────────────────────────────
    let cost_mentor          = i.mentor_hours_per_month * i.mentor_months * i.mentor_hourly_rate;
    let cost_training_total  = i.training_direct_cost + cost_mentor;

    // ── Горизонт + вероятность удержания ─────────────────────────────────────
    // Если вероятность удержания за horizon = p, то вероятность того,
    // что сотрудник всё же уйдёт и придётся заменять = (1 − p).
    // При коротком горизонте (< 1 года) эффект обучения не окупается полностью.
    let prob_leaves          = 1.0 - i.retention_probability.clamp(0.0, 1.0);
    let expected_replacement = cost_replacement * (1.0 - prob_leaves)
        + cost_replacement * prob_leaves * i.retention_horizon_years.max(0.1);
    // Упрощение: если сотрудник уходит раньше горизонта, замена происходит
    // пропорционально оставшемуся времени. Для горизонта = 1 год и p = 0 →
    // expected = cost_replacement (нейтральный случай).
    let expected_replacement_cost = cost_replacement
        * (i.retention_probability + (1.0 - i.retention_probability) * i.retention_horizon_years);

    // ── ROI ───────────────────────────────────────────────────────────────────
    let net_saving   = expected_replacement_cost - cost_training_total;
    let roi_percent  = if cost_training_total > 0.0 {
        net_saving / cost_training_total * 100.0
    } else {
        0.0
    };

    // ── Вердикт ───────────────────────────────────────────────────────────────
    let diff_ratio = net_saving.abs() / cost_training_total.max(1.0);
    let verdict = if diff_ratio < 0.05 {
        Verdict::Neutral
    } else if net_saving > 0.0 {
        Verdict::Train
    } else {
        Verdict::Hire
    };

    CalcResult {
        cost_replacement,
        cost_training_total,
        cost_training_direct: i.training_direct_cost,
        cost_mentor,
        expected_replacement_cost,
        roi_percent,
        net_saving,
        verdict,
        breakdown_recruitment:       recruitment,
        breakdown_productivity_loss: productivity_loss,
    }
}

// ─── Тесты ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn base_input() -> CalcInput {
        CalcInput {
            salary:                  24000.0,
            replacement_factor:      0.75,
            onboarding_months:       3.0,
            productivity_loss:       0.40,
            training_direct_cost:    1200.0,
            mentor_hours_per_month:  20.0,
            mentor_months:           2.0,
            mentor_hourly_rate:      25.0,
            retention_horizon_years: 2.0,
            retention_probability:   0.80,
        }
    }

    #[test]
    fn replacement_cost_correct() {
        let i = base_input();
        let r = compute(&i);
        // recruitment = 24000 * 0.75 = 18000
        // prod_loss   = 3 * 2000 * 0.40 = 2400
        // total       = 20400
        assert!((r.cost_replacement - 20400.0).abs() < 0.01);
    }

    #[test]
    fn mentor_cost_correct() {
        let i = base_input();
        let r = compute(&i);
        // 20h * 2 months * $25 = $1000
        assert!((r.cost_mentor - 1000.0).abs() < 0.01);
    }

    #[test]
    fn training_total_correct() {
        let i = base_input();
        let r = compute(&i);
        // 1200 + 1000 = 2200
        assert!((r.cost_training_total - 2200.0).abs() < 0.01);
    }

    #[test]
    fn train_verdict_for_base() {
        let i = base_input();
        let r = compute(&i);
        assert!(matches!(r.verdict, Verdict::Train));
    }

    #[test]
    fn hire_verdict_when_training_expensive() {
        let mut i = base_input();
        i.training_direct_cost    = 50000.0;
        i.mentor_hours_per_month  = 80.0;
        i.mentor_hourly_rate      = 100.0;
        i.retention_probability   = 0.10; // сотрудник скорее всего уйдёт
        let r = compute(&i);
        assert!(matches!(r.verdict, Verdict::Hire));
    }

    #[test]
    fn roi_zero_when_costs_equal() {
        let mut i = base_input();
        // подгоняем training под exact равенство
        let r0 = compute(&i);
        i.training_direct_cost = r0.expected_replacement_cost;
        i.mentor_hours_per_month = 0.0;
        let r = compute(&i);
        assert!(r.roi_percent.abs() < 1.0);
    }
}
