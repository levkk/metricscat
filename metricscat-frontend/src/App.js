import './App.css';
import React from 'react';

import { LineChart, Line, Tooltip } from 'recharts';

class App extends React.Component {

  state = {
    cpuUtilization: [],
    memoryUsed: [],
    memoryAvailable: [],
  }

  fetchData = () => {
    fetch("http://localhost:8000/api/metrics?name=system.cpu.utilization")
    .then((response) => response.json())
    .then((data) => this.setState({cpuUtilization: data}));

    fetch("http://localhost:8000/api/metrics?name=system.mem.used")
    .then((response) => response.json())
    .then((data) => this.setState({ memoryUsed: data}));
  }

  componentDidMount() {
    setInterval(this.fetchData, 1000);
  }

  render() {
    return (
      <div className="App" style={{
        padding: 25
      }}>
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
      </div>
    );
  }
}

export default App;
