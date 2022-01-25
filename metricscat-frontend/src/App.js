import './App.css';
import React from 'react';

import { LineChart, Line, Tooltip } from 'recharts';

function LogLine(props) {
  return (
    <tr>
      <td><pre>{props.line.recorded_at}</pre></td>
      <td><pre>{props.line.line}</pre></td>
    </tr>
  );
}

class App extends React.Component {

  state = {
    cpuUtilization: [],
    memoryUsed: [],
    memoryAvailable: [],
    logs: [],
    logsOffset: null,
    interval: 0,
  }

  fetchData = () => {
    fetch("http://localhost:8000/api/metrics?name=system.cpu.utilization")
    .then((response) => response.json())
    .then((data) => this.setState({cpuUtilization: data}));

    fetch("http://localhost:8000/api/metrics?name=system.mem.used")
    .then((response) => response.json())
    .then((data) => this.setState({ memoryUsed: data}));

    let offset = this.state.logsOffset ? `offset=${this.state.logsOffset}` : "";

    fetch(`http://localhost:8000/api/logs?${offset}`)
    .then((response) => response.json())
    .then((data) => {
      if (data.length > 0) {
        let offset = data[0].offset;
        let last_logs = this.state.logs.slice(0, Math.min(this.state.logs.length, 25));
        let logs = data.concat(last_logs);
        this.setState({ logs: logs, logsOffset: offset });
      }
    });
  }

  componentDidMount() {
    const interval = setInterval(this.fetchData, 1000);
    this.setState({interval: interval});
  }

  componentWillUnmount() {
    clearInterval(this.state.interval);
  }

  render() {
    return (
      <div className="App" style={{
        padding: 25
      }}>
        <h2>Metrics</h2>
        <h3>
          <pre style={{ width: 100 }}>system.cpu.utilization</pre>
        </h3>
        <LineChart width={600} height={300} data={this.state.cpuUtilization}>
          <Tooltip label="name" payload="name" labelFormatter={() => "system.cpu.utilization"} formatter={(value) => `${value}`} />
          <Line type="monotone" dataKey="value" stroke="#8884d8" isAnimationActive={false} dot={false} />
        </LineChart>
        <h3>
          <pre style={{ width: 100 }}>system.mem.used</pre>
        </h3>
        <LineChart width={600} height={300} data={this.state.memoryUsed}>
          <Tooltip label="name" payload="name" labelFormatter={() => "system.mem.used"} formatter={(value) => `${value}`} />
          <Line type="monotone" dataKey="value" stroke="#8884d8" isAnimationActive={false} dot={false} />
        </LineChart>

        <h2>Logs (live)</h2>
        <table>
          <thead>
            <tr>
              <th>Timestamp</th>
              <th>Line</th>
            </tr>
          </thead>
          <tbody>
            {this.state.logs.map((line) => <LogLine line={line} />)}
          </tbody>
        </table>
      </div>
    );
  }
}

export default App;
