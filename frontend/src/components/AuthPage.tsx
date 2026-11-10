import { useState, type FormEvent } from "react";
import { api, type User } from "../api";
import { LogoMark } from "./Icons";
import { ThemeToggle } from "./ThemeToggle";

export function AuthPage({ onAuth }: { onAuth: (token: string, user: User) => void }) {
  const [mode, setMode] = useState<"login" | "register">("login");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const call = mode === "login" ? api.login : api.register;
      const res = await call(email, password);
      onAuth(res.token, res.user);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Something went wrong";
      setError(mode === "login" && message === "unauthorized" ? "Wrong email or password." : message);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="auth">
      <div className="auth-theme">
        <ThemeToggle />
      </div>

      <section className="auth-pitch">
        <div className="brand">
          <LogoMark size={36} />
          <span>PriceWatch</span>
        </div>
        <h1>
          Stop refreshing store pages.
          <br />
          <em>Get told when the price drops.</em>
        </h1>
        <ul>
          <li>Paste a product link from any online store</li>
          <li>Set the price you're willing to pay</li>
          <li>We check it around the clock and keep the full history</li>
        </ul>
      </section>

      <form className="auth-card" onSubmit={submit}>
        <h2>{mode === "login" ? "Welcome back" : "Create your account"}</h2>

        <label>
          Email
          <input
            type="email"
            autoComplete="email"
            required
            autoFocus
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="you@example.com"
          />
        </label>
        <label>
          Password
          <input
            type="password"
            autoComplete={mode === "login" ? "current-password" : "new-password"}
            required
            minLength={8}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder="At least 8 characters"
          />
        </label>

        {error && (
          <p className="form-error" role="alert">
            {error}
          </p>
        )}

        <button className="btn btn-primary btn-lg" disabled={busy}>
          {busy ? "Please wait…" : mode === "login" ? "Log in" : "Create account"}
        </button>

        <p className="auth-switch">
          {mode === "login" ? "New here?" : "Already have an account?"}{" "}
          <button
            type="button"
            className="link"
            onClick={() => {
              setMode(mode === "login" ? "register" : "login");
              setError(null);
            }}
          >
            {mode === "login" ? "Create an account" : "Log in"}
          </button>
        </p>
      </form>
    </div>
  );
}
