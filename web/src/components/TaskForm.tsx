import React, { useState, useEffect } from 'react';
import {
  Box,
  TextField,
  Button,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Paper,
  Typography,
  Grid,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Tabs,
  Tab,
} from '@mui/material';
import axios from 'axios';

interface TaskConfig {
  url: string;
  method: string;
  headers: Record<string, string>;
  query_params: Record<string, string>;
  payload_template: any;
  duration: number;
  random_fields: string[];
}

interface ClientInfo {
  id: string;
  connected_at: string;
  last_active: string;
  stats: {
    total_requests: number;
    success_count: number;
    error_count: number;
    avg_response_time: number;
  };
}

const TaskForm: React.FC = () => {
  const [config, setConfig] = useState<TaskConfig>({
    url: '',
    method: 'GET',
    headers: {},
    query_params: {},
    payload_template: null,
    duration: 60,
    random_fields: [],
  });

  const [clientId, setClientId] = useState('client-1');
  const [status, setStatus] = useState<string>('');
  const [clients, setClients] = useState<ClientInfo[]>([]);
  const [tabValue, setTabValue] = useState(0);

  useEffect(() => {
    const fetchClients = async () => {
      try {
        const response = await axios.get('http://localhost:8080/clients');
        setClients(response.data);
      } catch (error) {
        console.error('获取客户端列表失败:', error);
      }
    };

    fetchClients();
    const interval = setInterval(fetchClients, 5000); // 每5秒更新一次
    return () => clearInterval(interval);
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      const response = await axios.post(
        `http://localhost:8080/assign/${clientId}`,
        config,
        {
          headers: {
            'Content-Type': 'application/json',
          },
        }
      );
      setStatus('任务下发成功！');
    } catch (error) {
      setStatus('任务下发失败：' + (error as Error).message);
    }
  };

  const handleSubmitAll = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      const response = await axios.post(
        'http://localhost:8080/assign_all',
        config,
        {
          headers: {
            'Content-Type': 'application/json',
          },
        }
      );
      setStatus(response.data.message);
    } catch (error) {
      setStatus('任务下发失败：' + (error as Error).message);
    }
  };

  const handleChange = (field: keyof TaskConfig, value: any) => {
    setConfig((prev) => ({
      ...prev,
      [field]: value,
    }));
  };

  const handleTabChange = (event: React.SyntheticEvent, newValue: number) => {
    setTabValue(newValue);
  };

  return (
    <Box sx={{ width: '100%' }}>
      <Tabs value={tabValue} onChange={handleTabChange} sx={{ mb: 3 }}>
        <Tab label="任务配置" />
        <Tab label="客户端列表" />
      </Tabs>

      {tabValue === 0 && (
        <Paper elevation={3} sx={{ p: 3, maxWidth: 600, mx: 'auto', mt: 4 }}>
          <Typography variant="h5" gutterBottom>
            压测任务配置
          </Typography>
          <form onSubmit={handleSubmit}>
            <Grid container spacing={2}>
              <Grid item xs={12}>
                <TextField
                  fullWidth
                  label="目标URL"
                  value={config.url}
                  onChange={(e) => handleChange('url', e.target.value)}
                  required
                />
              </Grid>
              <Grid item xs={12}>
                <FormControl fullWidth>
                  <InputLabel>HTTP方法</InputLabel>
                  <Select
                    value={config.method}
                    label="HTTP方法"
                    onChange={(e) => handleChange('method', e.target.value)}
                  >
                    <MenuItem value="GET">GET</MenuItem>
                    <MenuItem value="POST">POST</MenuItem>
                    <MenuItem value="PUT">PUT</MenuItem>
                    <MenuItem value="DELETE">DELETE</MenuItem>
                  </Select>
                </FormControl>
              </Grid>
              <Grid item xs={12}>
                <TextField
                  fullWidth
                  label="请求头 (JSON格式)"
                  multiline
                  rows={3}
                  value={JSON.stringify(config.headers, null, 2)}
                  onChange={(e) => {
                    try {
                      handleChange('headers', JSON.parse(e.target.value));
                    } catch (error) {
                      // 忽略无效的JSON
                    }
                  }}
                />
              </Grid>
              <Grid item xs={12}>
                <TextField
                  fullWidth
                  label="查询参数 (JSON格式)"
                  multiline
                  rows={3}
                  value={JSON.stringify(config.query_params, null, 2)}
                  onChange={(e) => {
                    try {
                      handleChange('query_params', JSON.parse(e.target.value));
                    } catch (error) {
                      // 忽略无效的JSON
                    }
                  }}
                />
              </Grid>
              <Grid item xs={12}>
                <TextField
                  fullWidth
                  label="请求体模板 (JSON格式)"
                  multiline
                  rows={4}
                  value={JSON.stringify(config.payload_template, null, 2)}
                  onChange={(e) => {
                    try {
                      handleChange('payload_template', JSON.parse(e.target.value));
                    } catch (error) {
                      // 忽略无效的JSON
                    }
                  }}
                />
              </Grid>
              <Grid item xs={12}>
                <TextField
                  fullWidth
                  type="number"
                  label="持续时间 (秒)"
                  value={config.duration}
                  onChange={(e) => handleChange('duration', parseInt(e.target.value))}
                />
              </Grid>
              <Grid item xs={12}>
                <TextField
                  fullWidth
                  label="随机字段 (逗号分隔)"
                  value={config.random_fields.join(', ')}
                  onChange={(e) => handleChange('random_fields', e.target.value.split(',').map(s => s.trim()))}
                />
              </Grid>
              <Grid item xs={12}>
                <Button
                  type="submit"
                  variant="contained"
                  color="primary"
                  fullWidth
                  size="large"
                >
                  下发任务
                </Button>
              </Grid>
              <Grid item xs={12}>
                <Button
                  onClick={handleSubmitAll}
                  variant="contained"
                  color="secondary"
                  fullWidth
                  size="large"
                >
                  下发到所有客户端
                </Button>
              </Grid>
              {status && (
                <Grid item xs={12}>
                  <Typography color={status.includes('成功') ? 'success.main' : 'error.main'}>
                    {status}
                  </Typography>
                </Grid>
              )}
            </Grid>
          </form>
        </Paper>
      )}

      {tabValue === 1 && (
        <Paper elevation={3} sx={{ p: 3, mx: 'auto', mt: 4 }}>
          <Typography variant="h5" gutterBottom>
            客户端列表
          </Typography>
          <TableContainer>
            <Table>
              <TableHead>
                <TableRow>
                  <TableCell>客户端ID</TableCell>
                  <TableCell>连接时间</TableCell>
                  <TableCell>最后活跃</TableCell>
                  <TableCell>总请求数</TableCell>
                  <TableCell>成功数</TableCell>
                  <TableCell>失败数</TableCell>
                  <TableCell>平均响应时间(ms)</TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {clients.map((client) => (
                  <TableRow key={client.id}>
                    <TableCell>{client.id}</TableCell>
                    <TableCell>{new Date(client.connected_at).toLocaleString()}</TableCell>
                    <TableCell>{new Date(client.last_active).toLocaleString()}</TableCell>
                    <TableCell>{client.stats.total_requests}</TableCell>
                    <TableCell>{client.stats.success_count}</TableCell>
                    <TableCell>{client.stats.error_count}</TableCell>
                    <TableCell>{client.stats.avg_response_time.toFixed(2)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </TableContainer>
        </Paper>
      )}
    </Box>
  );
};

export default TaskForm; 