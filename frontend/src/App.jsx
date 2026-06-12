import { useEffect, useState } from 'react'

export default function App() {
  const [health, setHealth] = useState(null)
  const [name, setName] = useState('World')
  const [greeting, setGreeting] = useState(null)

  useEffect(() => {
    fetch('/actuator/health')
      .then((r) => r.json())
      .then(setHealth)
      .catch(() => setHealth({ status: 'DOWN' }))
  }, [])

  const greet = async () => {
    const r = await fetch(`/api/greet/${encodeURIComponent(name)}`)
    setGreeting(await r.json())
  }

  return (
    <main style={{ fontFamily: 'system-ui', maxWidth: 640, margin: '4rem auto', padding: '0 1rem' }}>
      <h1>rustspring + React</h1>
      <p>
        Backend:{' '}
        <strong>{health ? `${health.status} (profile: ${health.profile ?? '?'})` : 'checking…'}</strong>
      </p>
      <div style={{ display: 'flex', gap: '0.5rem' }}>
        <input value={name} onChange={(e) => setName(e.target.value)} />
        <button onClick={greet}>Greet</button>
      </div>
      {greeting && (
        <pre style={{ background: '#f4f4f4', padding: '1rem', borderRadius: 8 }}>
          {JSON.stringify(greeting, null, 2)}
        </pre>
      )}
    </main>
  )
}
