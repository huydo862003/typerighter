export function addDocumentClass (condition: string, className: string): string {
  return `if(${condition})document.documentElement.classList.add(${JSON.stringify(className)});`;
}

export function readLocalStorage (key: string): string {
  return `localStorage.getItem(${JSON.stringify(key)})`;
}

export function setDocumentCssVariable (name: string, valueExpr: string): string {
  return `document.documentElement.style.setProperty(${JSON.stringify(name)},${valueExpr});`;
}

// Errors are silently swallowed - callers must not rely on effects from a failed body
export function wrapHeadScript (body: string): string {
  return `try{${body}}catch{}`;
}
