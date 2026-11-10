import { useEffect, useState } from "react";
import { api, setUnauthorizedHandler, token, type User } from "./api";
import { AuthPage } from "./components/AuthPage";
import { Dashboard } from "./components/Dashboard";
import { ToastProvider } from "./toast";

export function App() {
  const [user, setUser] = useState<User | null>(null);
  // While we check a saved token, show nothing rather than flashing the login form.
  const [loading, setLoading] = useState(() => token.get() !== null);

  const logout = () => {
    token.set(null);
    setUser(null);
  };

  useEffect(() => {
    setUnauthorizedHandler(logout);
    if (!token.get()) return;
    api
      .me()
      .then(setUser)
      .catch(() => token.set(null))
      .finally(() => setLoading(false));
  }, []);

  if (loading) return <div className="splash" aria-busy="true" />;

  return (
    <ToastProvider>
      {user ? (
        <Dashboard user={user} onLogout={logout} />
      ) : (
        <AuthPage
          onAuth={(t, u) => {
            token.set(t);
            setUser(u);
          }}
        />
      )}
    </ToastProvider>
  );
}
