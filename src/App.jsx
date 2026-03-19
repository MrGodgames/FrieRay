import { Routes, Route } from "react-router-dom";
import Layout from "./components/Layout/Layout";
import Dashboard from "./pages/Dashboard";
import Servers from "./pages/Servers";
import SplitTunnel from "./pages/SplitTunnel";
import Routing from "./pages/Routing";
import Logs from "./pages/Logs";
import AntiThrottle from "./pages/AntiThrottle";
import Settings from "./pages/Settings";

function App() {
  return (
    <Layout>
      <Routes>
        <Route path="/" element={<Dashboard />} />
        <Route path="/servers" element={<Servers />} />
        <Route path="/split-tunnel" element={<SplitTunnel />} />
        <Route path="/anti-throttle" element={<AntiThrottle />} />
        <Route path="/routing" element={<Routing />} />
        <Route path="/logs" element={<Logs />} />
        <Route path="/settings" element={<Settings />} />
      </Routes>
    </Layout>
  );
}

export default App;
