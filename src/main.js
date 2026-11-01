import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import { api } from './api.js';

mount(App, { target: document.getElementById('app') });
setTimeout(() => api.ready(), 60); // the window starts hidden; show it once the first frame exists
