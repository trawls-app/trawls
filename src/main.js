import 'mutationobserver-shim'
import Vue from 'vue'
import './plugins/bootstrap-vue'
import imageZoom from 'vue-image-zoomer';
import App from './App.vue'

Vue.config.productionTip = false



new Vue({
  render: h => h(App),
    components:{
      imageZoom
    }
}).$mount('#app')

const ImageZoom = require('vue-image-zoomer').default;
Vue.component('image-zoom', ImageZoom);