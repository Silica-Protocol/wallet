//! Chart rendering module for beautiful data visualization
//! 
//! This module provides high-performance chart rendering capabilities
//! using Canvas API for financial charts and network visualizations.

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use js_sys::{Promise, Object, Reflect, Array, Float64Array};
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d, Window, Document};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::{WasmError, WasmResult, rust_to_js, js_to_rust};

/// Chart types supported by the renderer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChartType {
    Line,
    Bar,
    Candlestick,
    Pie,
    Area,
    Scatter,
    Histogram,
}

impl ChartType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChartType::Line => "line",
            ChartType::Bar => "bar",
            ChartType::Candlestick => "candlestick",
            ChartType::Pie => "pie",
            ChartType::Area => "area",
            ChartType::Scatter => "scatter",
            ChartType::Histogram => "histogram",
        }
    }
}

/// Chart data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub x: f64,
    pub y: f64,
    pub label: Option<String>,
    pub color: Option<String>,
}

/// Chart data series
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSeries {
    pub name: String,
    pub data: Vec<DataPoint>,
    pub color: String,
    pub stroke_width: Option<f64>,
}

/// Chart configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartConfig {
    pub width: u32,
    pub height: u32,
    pub title: Option<String>,
    pub x_label: Option<String>,
    pub y_label: Option<String>,
    pub background_color: String,
    pub grid_color: String,
    pub text_color: String,
    pub padding: f64,
    pub animation_duration: u32,
    pub show_legend: bool,
    pub show_grid: bool,
}

/// Chart renderer for high-performance visualizations
#[wasm_bindgen]
pub struct ChartRenderer {
    canvas: Option<HtmlCanvasElement>,
    context: Option<CanvasRenderingContext2d>,
    config: ChartConfig,
    animation_frame: Option<i32>,
}

#[wasm_bindgen]
impl ChartRenderer {
    /// Create a new chart renderer
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: String, config_js: &JsValue) -> Result<ChartRenderer, JsValue> {
        let config: ChartConfig = js_to_rust(config_js)?;

        // Get canvas element
        let window = web_sys::window()
            .ok_or_else(|| JsValue::from_str("No window object"))?;
        let document = window.document()
            .ok_or_else(|| JsValue::from_str("No document object"))?;
        
        let canvas = document.get_element_by_id(&canvas_id)
            .ok_or_else(|| JsValue::from_str(&format!("Canvas element '{}' not found", canvas_id)))?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| JsValue::from_str("Element is not a canvas"))?;

        // Get 2D context
        let context = canvas.get_context("2d")
            .map_err(|_| JsValue::from_str("Failed to get 2D context"))?
            .ok_or_else(|| JsValue::from_str("No 2D context available"))?
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|_| JsValue::from_str("Context is not 2D"))?;

        // Set canvas size
        canvas.set_width(config.width);
        canvas.set_height(config.height);

        Ok(ChartRenderer {
            canvas: Some(canvas),
            context: Some(context),
            config,
            animation_frame: None,
        })
    }

    /// Draw a line chart
    #[wasm_bindgen]
    pub fn draw_line_chart(&mut self, series_js: &JsValue) -> Result<Promise, JsValue> {
        let series: Vec<DataSeries> = js_to_rust(series_js)?;
        
        let context = self.context.as_ref().unwrap().clone();
        let config = self.config.clone();

        let promise = future_to_promise(async move {
            draw_line_chart_internal(&context, &series, &config).await?;
            Ok(JsValue::UNDEFINED)
        });

        Ok(promise)
    }

    /// Draw a candlestick chart
    #[wasm_bindgen]
    pub fn draw_candlestick_chart(&mut self, data_js: &JsValue) -> Result<Promise, JsValue> {
        let data: Vec<CandlestickData> = js_to_rust(data_js)?;
        
        let context = self.context.as_ref().unwrap().clone();
        let config = self.config.clone();

        let promise = future_to_promise(async move {
            draw_candlestick_chart_internal(&context, &data, &config).await?;
            Ok(JsValue::UNDEFINED)
        });

        Ok(promise)
    }

    /// Draw a pie chart
    #[wasm_bindgen]
    pub fn draw_pie_chart(&mut self, data_js: &JsValue) -> Result<Promise, JsValue> {
        let data: Vec<PieData> = js_to_rust(data_js)?;
        
        let context = self.context.as_ref().unwrap().clone();
        let config = self.config.clone();

        let promise = future_to_promise(async move {
            draw_pie_chart_internal(&context, &data, &config).await?;
            Ok(JsValue::UNDEFINED)
        });

        Ok(promise)
    }

    /// Draw a bar chart
    #[wasm_bindgen]
    pub fn draw_bar_chart(&mut self, series_js: &JsValue) -> Result<Promise, JsValue> {
        let series: Vec<DataSeries> = js_to_rust(series_js)?;
        
        let context = self.context.as_ref().unwrap().clone();
        let config = self.config.clone();

        let promise = future_to_promise(async move {
            draw_bar_chart_internal(&context, &series, &config).await?;
            Ok(JsValue::UNDEFINED)
        });

        Ok(promise)
    }

    /// Clear the canvas
    #[wasm_bindgen]
    pub fn clear(&self) -> Result<(), JsValue> {
        let context = self.context.as_ref().unwrap();
        let canvas = self.canvas.as_ref().unwrap();
        
        context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
        
        // Fill background
        context.set_fill_style(&self.config.background_color.into());
        context.fill_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
        
        Ok(())
    }

    /// Update chart configuration
    #[wasm_bindgen]
    pub fn update_config(&mut self, config_js: &JsValue) -> Result<(), JsValue> {
        let new_config: ChartConfig = js_to_rust(config_js)?;
        
        // Update canvas size if needed
        if let Some(canvas) = &self.canvas {
            if canvas.width() != new_config.width || canvas.height() != new_config.height {
                canvas.set_width(new_config.width);
                canvas.set_height(new_config.height);
            }
        }
        
        self.config = new_config;
        Ok(())
    }

    /// Export chart as image data URL
    #[wasm_bindgen]
    pub fn export_as_image(&self, format: String) -> Result<String, JsValue> {
        let canvas = self.canvas.as_ref().unwrap();
        
        let data_url = match format.as_str() {
            "png" => canvas.to_data_url_with_type("image/png")
                .map_err(|_| JsValue::from_str("Failed to export as PNG"))?,
            "jpeg" => canvas.to_data_url_with_type("image/jpeg")
                .map_err(|_| JsValue::from_str("Failed to export as JPEG"))?,
            "webp" => canvas.to_data_url_with_type("image/webp")
                .map_err(|_| JsValue::from_str("Failed to export as WebP"))?,
            _ => return Err(WasmError::new("INVALID_FORMAT", "Supported formats: png, jpeg, webp").into()),
        };
        
        Ok(data_url)
    }

    /// Add animation to chart
    #[wasm_bindgen]
    pub fn animate(&mut self, duration_ms: u32) -> Result<(), JsValue> {
        // Cancel any existing animation
        if let Some(frame) = self.animation_frame {
            web_sys::window()
                .unwrap()
                .cancel_animation_frame(frame)
                .unwrap();
        }

        let start_time = web_sys::window()
            .unwrap()
            .performance()
            .unwrap()
            .now();

        // This would need to be implemented with proper animation callbacks
        // For now, we'll just set the duration in config
        self.config.animation_duration = duration_ms;

        Ok(())
    }
}

/// Candlestick data for financial charts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandlestickData {
    pub timestamp: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// Pie chart data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PieData {
    pub label: String,
    pub value: f64,
    pub color: String,
}

// Internal drawing functions
async fn draw_line_chart_internal(
    context: &CanvasRenderingContext2d,
    series: &[DataSeries],
    config: &ChartConfig,
) -> WasmResult<()> {
    // Clear canvas
    context.clear_rect(0.0, 0.0, config.width as f64, config.height as f64);
    
    // Set background
    context.set_fill_style(&config.background_color.into());
    context.fill_rect(0.0, 0.0, config.width as f64, config.height as f64);

    // Draw grid if enabled
    if config.show_grid {
        draw_grid(context, config)?;
    }

    // Calculate data bounds
    let (min_x, max_x, min_y, max_y) = calculate_data_bounds(series);

    // Draw axes
    draw_axes(context, config, min_x, max_x, min_y, max_y)?;

    // Draw each series
    for data_series in series {
        draw_line_series(context, data_series, config, min_x, max_x, min_y, max_y)?;
    }

    // Draw legend if enabled
    if config.show_legend {
        draw_legend(context, series, config)?;
    }

    // Draw title
    if let Some(ref title) = config.title {
        draw_title(context, title, config)?;
    }

    Ok(())
}

async fn draw_candlestick_chart_internal(
    context: &CanvasRenderingContext2d,
    data: &[CandlestickData],
    config: &ChartConfig,
) -> WasmResult<()> {
    // Clear canvas
    context.clear_rect(0.0, 0.0, config.width as f64, config.height as f64);
    
    // Set background
    context.set_fill_style(&config.background_color.into());
    context.fill_rect(0.0, 0.0, config.width as f64, config.height as f64);

    if data.is_empty() {
        return Ok(());
    }

    // Calculate bounds
    let min_price = data.iter().map(|d| d.low).fold(f64::INFINITY, f64::min);
    let max_price = data.iter().map(|d| d.high).fold(f64::NEG_INFINITY, f64::max);
    let min_time = data.first().unwrap().timestamp;
    let max_time = data.last().unwrap().timestamp;

    // Draw grid
    if config.show_grid {
        draw_grid(context, config)?;
    }

    // Draw axes
    draw_axes(context, config, min_time, max_time, min_price, max_price)?;

    // Draw candlesticks
    let chart_width = config.width as f64 - 2.0 * config.padding;
    let chart_height = config.height as f64 - 2.0 * config.padding;
    let candle_width = chart_width / data.len() as f64 * 0.6;

    for (i, candle) in data.iter().enumerate() {
        let x = config.padding + (i as f64 / data.len() as f64) * chart_width;
        
        // Calculate y positions
        let y_open = config.padding + (1.0 - (candle.open - min_price) / (max_price - min_price)) * chart_height;
        let y_close = config.padding + (1.0 - (candle.close - min_price) / (max_price - min_price)) * chart_height;
        let y_high = config.padding + (1.0 - (candle.high - min_price) / (max_price - min_price)) * chart_height;
        let y_low = config.padding + (1.0 - (candle.low - min_price) / (max_price - min_price)) * chart_height;

        // Set color based on price movement
        let color = if candle.close >= candle.open {
            "#00ff00" // Green for bullish
        } else {
            "#ff0000" // Red for bearish
        };

        // Draw high-low line
        context.set_stroke_style(&color.into());
        context.set_line_width(1.0);
        context.begin_path();
        context.move_to(x, y_high);
        context.line_to(x, y_low);
        context.stroke();

        // Draw open-close box
        context.set_fill_style(&color.into());
        let box_top = y_open.min(y_close);
        let box_height = (y_close - y_open).abs();
        
        if candle.close >= candle.open {
            // Filled box for bullish
            context.fill_rect(x - candle_width / 2.0, box_top, candle_width, box_height);
        } else {
            // Hollow box for bearish
            context.stroke_rect(x - candle_width / 2.0, box_top, candle_width, box_height);
        }
    }

    Ok(())
}

async fn draw_pie_chart_internal(
    context: &CanvasRenderingContext2d,
    data: &[PieData],
    config: &ChartConfig,
) -> WasmResult<()> {
    // Clear canvas
    context.clear_rect(0.0, 0.0, config.width as f64, config.height as f64);
    
    // Set background
    context.set_fill_style(&config.background_color.into());
    context.fill_rect(0.0, 0.0, config.width as f64, config.height as f64);

    if data.is_empty() {
        return Ok(());
    }

    // Calculate total
    let total: f64 = data.iter().map(|d| d.value).sum();

    // Calculate center and radius
    let center_x = config.width as f64 / 2.0;
    let center_y = config.height as f64 / 2.0;
    let radius = ((config.width as f64).min(config.height as f64) / 2.0 - config.padding).min(200.0);

    let mut current_angle = -std::f64::consts::PI / 2.0; // Start from top

    for pie_data in data {
        let slice_angle = (pie_data.value / total) * 2.0 * std::f64::consts::PI;

        // Draw slice
        context.set_fill_style(&pie_data.color.into());
        context.begin_path();
        context.move_to(center_x, center_y);
        context.arc(center_x, center_y, radius, current_angle, current_angle + slice_angle);
        context.close_path();
        context.fill();

        // Draw label
        let label_angle = current_angle + slice_angle / 2.0;
        let label_x = center_x + (radius * 0.7) * label_angle.cos();
        let label_y = center_y + (radius * 0.7) * label_angle.sin();

        context.set_fill_style(&config.text_color.into());
        context.set_font("12px sans-serif");
        context.set_text_align("center");
        context.set_text_baseline("middle");
        
        let percentage = (pie_data.value / total * 100.0).round();
        let label_text = format!("{} ({}%)", pie_data.label, percentage);
        context.fill_text(&label_text, label_x, label_y);

        current_angle += slice_angle;
    }

    // Draw legend
    if config.show_legend {
        draw_pie_legend(context, data, config)?;
    }

    Ok(())
}

async fn draw_bar_chart_internal(
    context: &CanvasRenderingContext2d,
    series: &[DataSeries],
    config: &ChartConfig,
) -> WasmResult<()> {
    // Clear canvas
    context.clear_rect(0.0, 0.0, config.width as f64, config.height as f64);
    
    // Set background
    context.set_fill_style(&config.background_color.into());
    context.fill_rect(0.0, 0.0, config.width as f64, config.height as f64);

    if series.is_empty() || series[0].data.is_empty() {
        return Ok(());
    }

    // Calculate bounds
    let (min_x, max_x, min_y, max_y) = calculate_data_bounds(series);

    // Draw grid
    if config.show_grid {
        draw_grid(context, config)?;
    }

    // Draw axes
    draw_axes(context, config, min_x, max_x, min_y, max_y)?;

    // Draw bars
    let chart_width = config.width as f64 - 2.0 * config.padding;
    let chart_height = config.height as f64 - 2.0 * config.padding;
    
    for data_series in series {
        let bar_width = chart_width / data_series.data.len() as f64 / series.len() as f64 * 0.8;
        
        for (i, data_point) in data_series.data.iter().enumerate() {
            let x = config.padding + (i as f64 / data_series.data.len() as f64) * chart_width 
                + (series.iter().position(|s| std::ptr::eq(s, data_series)).unwrap_or(0) as f64 * bar_width);
            
            let bar_height = ((data_point.y - min_y) / (max_y - min_y)) * chart_height;
            let y = config.padding + chart_height - bar_height;

            context.set_fill_style(&data_series.color.into());
            context.fill_rect(x, y, bar_width, bar_height);
        }
    }

    // Draw legend if enabled
    if config.show_legend {
        draw_legend(context, series, config)?;
    }

    Ok(())
}

// Utility functions
fn calculate_data_bounds(series: &[DataSeries]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for data_series in series {
        for data_point in &data_series.data {
            min_x = min_x.min(data_point.x);
            max_x = max_x.max(data_point.x);
            min_y = min_y.min(data_point.y);
            max_y = max_y.max(data_point.y);
        }
    }

    (min_x, max_x, min_y, max_y)
}

fn draw_grid(context: &CanvasRenderingContext2d, config: &ChartConfig) -> WasmResult<()> {
    context.set_stroke_style(&config.grid_color.into());
    context.set_line_width(0.5);

    let chart_width = config.width as f64 - 2.0 * config.padding;
    let chart_height = config.height as f64 - 2.0 * config.padding;

    // Vertical grid lines
    for i in 0..=10 {
        let x = config.padding + (i as f64 / 10.0) * chart_width;
        context.begin_path();
        context.move_to(x, config.padding);
        context.line_to(x, config.padding + chart_height);
        context.stroke();
    }

    // Horizontal grid lines
    for i in 0..=10 {
        let y = config.padding + (i as f64 / 10.0) * chart_height;
        context.begin_path();
        context.move_to(config.padding, y);
        context.line_to(config.padding + chart_width, y);
        context.stroke();
    }

    Ok(())
}

fn draw_axes(
    context: &CanvasRenderingContext2d,
    config: &ChartConfig,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
) -> WasmResult<()> {
    context.set_stroke_style(&config.text_color.into());
    context.set_line_width(2.0);

    let chart_width = config.width as f64 - 2.0 * config.padding;
    let chart_height = config.height as f64 - 2.0 * config.padding;

    // X-axis
    context.begin_path();
    context.move_to(config.padding, config.padding + chart_height);
    context.line_to(config.padding + chart_width, config.padding + chart_height);
    context.stroke();

    // Y-axis
    context.begin_path();
    context.move_to(config.padding, config.padding);
    context.line_to(config.padding, config.padding + chart_height);
    context.stroke();

    // Draw labels
    context.set_fill_style(&config.text_color.into());
    context.set_font("12px sans-serif");
    context.set_text_align("center");
    context.set_text_baseline("top");

    // X-axis label
    if let Some(ref x_label) = config.x_label {
        context.fill_text(
            x_label,
            config.padding + chart_width / 2.0,
            config.padding + chart_height + 20.0,
        );
    }

    // Y-axis label
    if let Some(ref y_label) = config.y_label {
        context.save();
        context.translate(config.padding - 30.0, config.padding + chart_height / 2.0);
        context.rotate(-std::f64::consts::PI / 2.0);
        context.fill_text(y_label, 0.0, 0.0);
        context.restore();
    }

    Ok(())
}

fn draw_line_series(
    context: &CanvasRenderingContext2d,
    series: &DataSeries,
    config: &ChartConfig,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
) -> WasmResult<()> {
    if series.data.is_empty() {
        return Ok(());
    }

    let chart_width = config.width as f64 - 2.0 * config.padding;
    let chart_height = config.height as f64 - 2.0 * config.padding;

    context.set_stroke_style(&series.color.into());
    context.set_line_width(series.stroke_width.unwrap_or(2.0));
    context.begin_path();

    for (i, data_point) in series.data.iter().enumerate() {
        let x = config.padding + ((data_point.x - min_x) / (max_x - min_x)) * chart_width;
        let y = config.padding + (1.0 - (data_point.y - min_y) / (max_y - min_y)) * chart_height;

        if i == 0 {
            context.move_to(x, y);
        } else {
            context.line_to(x, y);
        }
    }

    context.stroke();

    // Draw data points
    for data_point in &series.data {
        let x = config.padding + ((data_point.x - min_x) / (max_x - min_x)) * chart_width;
        let y = config.padding + (1.0 - (data_point.y - min_y) / (max_y - min_y)) * chart_height;

        context.set_fill_style(&series.color.into());
        context.begin_path();
        context.arc(x, y, 3.0, 0.0, 2.0 * std::f64::consts::PI);
        context.fill();
    }

    Ok(())
}

fn draw_legend(
    context: &CanvasRenderingContext2d,
    series: &[DataSeries],
    config: &ChartConfig,
) -> WasmResult<()> {
    let legend_x = config.width as f64 - config.padding - 150.0;
    let mut legend_y = config.padding + 20.0;

    context.set_font("12px sans-serif");
    context.set_text_align("left");
    context.set_text_baseline("middle");

    for data_series in series {
        // Draw color box
        context.set_fill_style(&data_series.color.into());
        context.fill_rect(legend_x, legend_y - 6.0, 12.0, 12.0);

        // Draw label
        context.set_fill_style(&config.text_color.into());
        context.fill_text(&data_series.name, legend_x + 20.0, legend_y);

        legend_y += 20.0;
    }

    Ok(())
}

fn draw_pie_legend(
    context: &CanvasRenderingContext2d,
    data: &[PieData],
    config: &ChartConfig,
) -> WasmResult<()> {
    let legend_x = config.width as f64 - config.padding - 150.0;
    let mut legend_y = config.padding + 20.0;

    context.set_font("12px sans-serif");
    context.set_text_align("left");
    context.set_text_baseline("middle");

    for pie_data in data {
        // Draw color box
        context.set_fill_style(&pie_data.color.into());
        context.fill_rect(legend_x, legend_y - 6.0, 12.0, 12.0);

        // Draw label
        context.set_fill_style(&config.text_color.into());
        context.fill_text(&pie_data.label, legend_x + 20.0, legend_y);

        legend_y += 20.0;
    }

    Ok(())
}

fn draw_title(context: &CanvasRenderingContext2d, title: &str, config: &ChartConfig) -> WasmResult<()> {
    context.set_fill_style(&config.text_color.into());
    context.set_font("bold 16px sans-serif");
    context.set_text_align("center");
    context.set_text_baseline("top");
    
    context.fill_text(
        title,
        config.width as f64 / 2.0,
        config.padding / 2.0,
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_chart_type() {
        let chart_type = ChartType::Line;
        assert_eq!(chart_type.as_str(), "line");
        
        let candlestick = ChartType::Candlestick;
        assert_eq!(candlestick.as_str(), "candlestick");
    }

    #[wasm_bindgen_test]
    fn test_data_bounds_calculation() {
        let series = vec![
            DataSeries {
                name: "Test".to_string(),
                data: vec![
                    DataPoint { x: 0.0, y: 10.0, label: None, color: None },
                    DataPoint { x: 1.0, y: 20.0, label: None, color: None },
                    DataPoint { x: 2.0, y: 15.0, label: None, color: None },
                ],
                color: "#000000".to_string(),
                stroke_width: None,
            }
        ];

        let (min_x, max_x, min_y, max_y) = calculate_data_bounds(&series);
        assert_eq!(min_x, 0.0);
        assert_eq!(max_x, 2.0);
        assert_eq!(min_y, 10.0);
        assert_eq!(max_y, 20.0);
    }
}