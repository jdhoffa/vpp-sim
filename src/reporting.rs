use crate::simulation::SimulationKpis;

pub fn print_kpi_report(kpis: &SimulationKpis) {
    println!("\n--- KPI Report ---");
    println!("RMSE tracking error: {:.3} kW", kpis.rmse_tracking_kw);
    println!("Curtailment achieved: {:.1}%", kpis.curtailment_pct);
    println!("Feeder peak load: {:.2} kW", kpis.feeder_peak_load_kw);
}
