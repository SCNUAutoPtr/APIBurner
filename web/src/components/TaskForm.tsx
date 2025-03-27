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
    current_qps: number;
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

  // 添加本地状态来存储 JSON 字符串
  const [jsonInputs, setJsonInputs] = useState({
    headers: '{}',
    query_params: '{}',
    payload_template: 'null'
  });

  // 添加错误状态
  const [jsonErrors, setJsonErrors] = useState({
    headers: '',
    query_params: '',
    payload_template: ''
  });

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
    const interval = setInterval(fetchClients, 1000); // 每秒更新一次
    return () => clearInterval(interval);
  }, []);

  // 更新 JSON 输入处理函数
  const handleJsonChange = (field: 'headers' | 'query_params' | 'payload_template', value: string) => {
    setJsonInputs(prev => ({ ...prev, [field]: value }));
    try {
      const parsed = JSON.parse(value);
      if (field === 'payload_template') {
        // 确保 payload_template 是一个对象
        if (typeof parsed === 'object' && parsed !== null) {
          handleChange(field, parsed);
          setJsonErrors(prev => ({ ...prev, [field]: '' }));
        } else {
          handleChange(field, null);
          setJsonErrors(prev => ({ ...prev, [field]: 'payload_template 必须是一个对象' }));
        }
      } else {
        handleChange(field, parsed);
        setJsonErrors(prev => ({ ...prev, [field]: '' }));
      }
    } catch (error) {
      // 保持当前值不变
      const errorMessage = error instanceof Error ? error.message : '未知错误';
      setJsonErrors(prev => ({ ...prev, [field]: `JSON 解析错误: ${errorMessage}` }));
      console.error(`JSON解析错误 (${field}):`, error);
    }
  };

  // 初始化 JSON 字符串
  useEffect(() => {
    setJsonInputs({
      headers: JSON.stringify(config.headers, null, 2),
      query_params: JSON.stringify(config.query_params, null, 2),
      payload_template: JSON.stringify(config.payload_template, null, 2)
    });
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      // 在发送前确保所有 JSON 都是有效的
      const finalConfig = {
        ...config,
        headers: JSON.parse(jsonInputs.headers),
        query_params: JSON.parse(jsonInputs.query_params),
        payload_template: JSON.parse(jsonInputs.payload_template)
      };

      const response = await axios.post(
        `http://localhost:8080/assign/${clientId}`,
        finalConfig,
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
      // 在发送前确保所有 JSON 都是有效的
      const finalConfig = {
        ...config,
        headers: JSON.parse(jsonInputs.headers),
        query_params: JSON.parse(jsonInputs.query_params),
        payload_template: JSON.parse(jsonInputs.payload_template)
      };

      const response = await axios.post(
        'http://localhost:8080/assign_all',
        finalConfig,
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

  // 添加 JSON 验证函数
  const isValidJson = (str: string): boolean => {
    try {
      JSON.parse(str);
      return true;
    } catch (e) {
      return false;
    }
  };

  return (
    <Box sx={{ p: 3 }}>
      <Grid container spacing={3}>
        {/* 左侧：任务配置 */}
        <Grid item xs={12} md={6}>
          <Paper sx={{ p: 2 }}>
            <Typography variant="h6" gutterBottom>
              任务配置
            </Typography>
            <form onSubmit={handleSubmit}>
              <Grid container spacing={2}>
                <Grid item xs={12}>
                  <TextField
                    fullWidth
                    label="URL"
                    value={config.url}
                    onChange={(e) => handleChange('url', e.target.value)}
                    required
                  />
                </Grid>
                <Grid item xs={12}>
                  <FormControl fullWidth>
                    <InputLabel>请求方法</InputLabel>
                    <Select
                      value={config.method}
                      label="请求方法"
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
                    value={jsonInputs.headers}
                    onChange={(e) => handleJsonChange('headers', e.target.value)}
                    error={!isValidJson(jsonInputs.headers)}
                    helperText={jsonErrors.headers || "例如: { 'Content-Type': 'application/json' }"}
                  />
                </Grid>
                <Grid item xs={12}>
                  <TextField
                    fullWidth
                    label="查询参数 (JSON格式)"
                    multiline
                    rows={3}
                    value={jsonInputs.query_params}
                    onChange={(e) => handleJsonChange('query_params', e.target.value)}
                    error={!isValidJson(jsonInputs.query_params)}
                    helperText={jsonErrors.query_params || "例如: { 'page': '1', 'size': '10' }"}
                  />
                </Grid>
                <Grid item xs={12}>
                  <TextField
                    fullWidth
                    label="请求体模板 (JSON格式)"
                    multiline
                    rows={4}
                    value={jsonInputs.payload_template}
                    onChange={(e) => handleJsonChange('payload_template', e.target.value)}
                    error={!isValidJson(jsonInputs.payload_template)}
                    helperText={jsonErrors.payload_template || "例如: { 'name': 'test', 'age': 18 }"}
                  />
                </Grid>
                <Grid item xs={12}>
                  <TextField
                    fullWidth
                    label="随机字段 (逗号分隔)"
                    value={config.random_fields.join(', ')}
                    onChange={(e) => handleChange('random_fields', e.target.value.split(',').map(s => s.trim()))}
                    helperText="例如: name, age, address"
                  />
                </Grid>
                <Grid item xs={12}>
                  <TextField
                    fullWidth
                    label="持续时间（秒）"
                    type="number"
                    value={config.duration}
                    onChange={(e) => handleChange('duration', parseInt(e.target.value))}
                    required
                  />
                </Grid>
                <Grid item xs={12}>
                  <Button
                    variant="contained"
                    color="primary"
                    type="submit"
                    fullWidth
                  >
                    发送任务
                  </Button>
                </Grid>
                <Grid item xs={12}>
                  <Button
                    variant="contained"
                    color="secondary"
                    onClick={handleSubmitAll}
                    fullWidth
                  >
                    发送到所有客户端
                  </Button>
                </Grid>
              </Grid>
            </form>
          </Paper>
        </Grid>

        {/* 右侧：客户端列表 */}
        <Grid item xs={12} md={6}>
          <Paper sx={{ p: 2 }}>
            <Typography variant="h6" gutterBottom>
              客户端状态
            </Typography>
            <TableContainer>
              <Table>
                <TableHead>
                  <TableRow>
                    <TableCell>客户端ID</TableCell>
                    <TableCell>连接时间</TableCell>
                    <TableCell>最后活跃</TableCell>
                    <TableCell>QPS</TableCell>
                    <TableCell>总请求数</TableCell>
                    <TableCell>成功率</TableCell>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {clients.map((client) => (
                    <TableRow key={client.id}>
                      <TableCell>{client.id}</TableCell>
                      <TableCell>{new Date(client.connected_at).toLocaleString()}</TableCell>
                      <TableCell>{new Date(client.last_active).toLocaleString()}</TableCell>
                      <TableCell>{client.stats.current_qps.toFixed(2)}</TableCell>
                      <TableCell>{client.stats.total_requests}</TableCell>
                      <TableCell>
                        {client.stats.total_requests > 0
                          ? ((client.stats.success_count / client.stats.total_requests) * 100).toFixed(2) + '%'
                          : '0%'}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </TableContainer>
          </Paper>
        </Grid>
      </Grid>

      {status && (
        <Paper sx={{ p: 2, mt: 2, bgcolor: 'background.default' }}>
          <Typography>{status}</Typography>
        </Paper>
      )}
    </Box>
  );
};

export default TaskForm; 