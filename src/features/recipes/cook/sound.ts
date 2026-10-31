/** A short two-tone chime for a finished timer. The browser may block it until the user has interacted with the page. */
export function playChime() {
	const context = new AudioContext();
	const start = context.currentTime;

	[880, 1175].forEach((frequency, index) => {
		const oscillator = context.createOscillator();
		const gain = context.createGain();
		const at = start + index * 0.3;
		oscillator.frequency.value = frequency;
		gain.gain.setValueAtTime(0.25, at);
		gain.gain.exponentialRampToValueAtTime(0.001, at + 0.25);
		oscillator.connect(gain).connect(context.destination);
		oscillator.start(at);
		oscillator.stop(at + 0.25);
	});

	setTimeout(() => void context.close(), 1000);
}
