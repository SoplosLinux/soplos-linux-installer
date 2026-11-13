import { writable } from "svelte/store";

// Mnemonics, as in GTK, Qt and Calamares: holding Alt underlines one letter
// of each button, and Alt + that letter presses it. The letters come from
// the labels in the language chosen, so they need no translation of their
// own: each button takes the first letter of a word that no button shown
// before it has taken, else any free letter of its label.

/** True while Alt is held: the letters are underlined. */
export const altHeld = writable(false);

/** The buttons on screen, by the letter that presses them. */
export const mnemonicActions = writable<Record<string, () => void>>({});

/** A letter as it is compared: lower case, without accents. */
export function keyOf(char: string): string {
  return char.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
}

const isLetter = (c: string) => /\p{L}/u.test(c);

/** For each label, the index of its letter, or -1 when none is free. */
export function assignMnemonics(labels: string[]): number[] {
  const used = new Set<string>();
  return labels.map((label) => {
    const starts: number[] = [];
    const rest: number[] = [];
    for (let i = 0; i < label.length; i++) {
      if (!isLetter(label[i])) continue;
      (i === 0 || !isLetter(label[i - 1]) ? starts : rest).push(i);
    }
    for (const i of [...starts, ...rest]) {
      const key = keyOf(label[i]);
      if (!used.has(key)) {
        used.add(key);
        return i;
      }
    }
    return -1;
  });
}
