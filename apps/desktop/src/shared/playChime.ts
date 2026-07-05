// Простой сигнал через Web Audio — без бандла аудио-файла и выбора звука
// (soundId из раздела 8 ТЗ). Кастомные звуки — отдельная будущая задача.
export function playChime() {
  try {
    const ctx = new AudioContext();
    const oscillator = ctx.createOscillator();
    const gain = ctx.createGain();
    oscillator.connect(gain);
    gain.connect(ctx.destination);
    oscillator.frequency.value = 880;
    gain.gain.setValueAtTime(0.2, ctx.currentTime);
    oscillator.start();
    oscillator.stop(ctx.currentTime + 0.3);
  } catch {
    // Web Audio недоступен (например, автоплей заблокирован политикой) —
    // алерт всё равно показывается визуально, звук не критичен для функции.
  }
}
