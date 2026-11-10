import { useState, type FormEvent } from "react";
import { api, type Product } from "../api";
import { useToast } from "../toast";
import { PlusIcon } from "./Icons";

export function AddProduct({ onAdded }: { onAdded: (p: Product) => void }) {
  const toast = useToast();
  const [url, setUrl] = useState("");
  const [target, setTarget] = useState("");
  const [name, setName] = useState("");
  const [selector, setSelector] = useState("");
  const [advanced, setAdvanced] = useState(false);
  const [busy, setBusy] = useState(false);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    try {
      const product = await api.createProduct({
        url: url.trim(),
        target_price: target.replace(",", ".").trim(),
        name: name.trim() || undefined,
        css_selector: selector.trim() || undefined,
      });
      onAdded(product);
      setUrl("");
      setTarget("");
      setName("");
      setSelector("");
      toast("ok", "Added. Fetching the first price…");
    } catch (err) {
      toast("error", err instanceof Error ? err.message : "Could not add the product");
    } finally {
      setBusy(false);
    }
  };

  return (
    <form className="card add" onSubmit={submit}>
      <div className="add-row">
        <label className="grow">
          <span>Product link</span>
          <input
            type="url"
            required
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="https://store.example/product/…"
          />
        </label>
        <label className="target">
          <span>Target price</span>
          <input
            inputMode="decimal"
            required
            pattern="[0-9]+([.,][0-9]{1,2})?"
            title="A number like 499 or 499.90"
            value={target}
            onChange={(e) => setTarget(e.target.value)}
            placeholder="499.00"
          />
        </label>
        <button className="btn btn-primary" disabled={busy}>
          <PlusIcon />
          {busy ? "Adding…" : "Track price"}
        </button>
      </div>

      <button type="button" className="link small" onClick={() => setAdvanced(!advanced)} aria-expanded={advanced}>
        {advanced ? "Hide options" : "More options"}
      </button>

      {advanced && (
        <div className="add-row">
          <label className="grow">
            <span>Name (optional)</span>
            <input value={name} onChange={(e) => setName(e.target.value)} placeholder="Taken from the page if empty" />
          </label>
          <label className="grow">
            <span>CSS selector (optional)</span>
            <input
              value={selector}
              onChange={(e) => setSelector(e.target.value)}
              placeholder=".product-price — only if the price isn't found automatically"
              spellCheck={false}
            />
          </label>
        </div>
      )}
    </form>
  );
}
