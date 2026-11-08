export function addDocumentClass (className: string, condition: string = 'true'): string {
  return `if(${condition})document.documentElement.classList.add(${JSON.stringify(className)});`;
}

export function readLocalStorage (key: string): string {
  return `localStorage.getItem(${JSON.stringify(key)})`;
}

export function setDocumentCssVariable (name: string, valueIfTrueExpr: string, condition: string = 'true'): string {
  return `if(${condition})document.documentElement.style.setProperty(${JSON.stringify(name)},${valueIfTrueExpr});`;
}

// Errors are silently swallowed - callers must not rely on effects from a failed body
export function wrapHeadScript (body: string): string {
  return `try{${body}}catch(e){console.log(e)}`;
}
