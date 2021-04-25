<template>
  <div>
    <form>
      <h4><b-icon icon="hdd"></b-icon> Output path</h4>
      <div class="form-group">

        <div class="input-group">
          <input class="form-control" type="text" :placeholder="output_path" id="out_path" readonly>
          <div class="input-group-append">
            <b-button v-on:click="choose_output" variant="primary">Choose output</b-button>
          </div>
        </div>

        <small id="out_path_help" class="form-text text-muted">Where the resulting DNG file should be saved.</small>
      </div>

      <h4><b-icon icon="sliders"></b-icon> Mode selection</h4>
      <div class="form-group form-check">
        <b-card-group deck class="col d-flex justify-content-center">
          <b-card
              title="Normal"
              img-src="@/assets/examples/mode_normal.jpg"
              img-top
              tag="article"
              style="max-width: 20rem;"
              border-variant="primary"
              class="mb-2"
          >
            <b-card-text>
              <div class="form-check">
                <input class="form-check-input" type="radio" id="normal_mode" v-model="merge_mode" v-bind:value="'normal'">
                <label class="form-check-label" for="normal_mode">All images will be weighted identically.</label>
              </div>
            </b-card-text>
          </b-card>

          <b-card
              title="Falling Comets"
              img-src="@/assets/examples/mode_falling.jpg"
              img-top
              tag="article"
              style="max-width: 20rem;"
              border-variant="primary"
              class="mb-2"
          >
            <b-card-text>
              <div class="form-check">
                <input class="form-check-input" type="radio" id="falling_mode" v-model="merge_mode" v-bind:value="'falling'">
                <label class="form-check-label" for="falling_mode">Later images become darker, such that stars will fade away.</label>
              </div>

            </b-card-text>
          </b-card>

          <b-card
              title="Raising Comets"
              img-src="@/assets/examples/mode_raising.jpg"
              img-top
              tag="article"
              style="max-width: 20rem;"
              border-variant="primary"
              class="mb-2"
          >
            <b-card-text>
              <div class="form-check">
                <input class="form-check-input" type="radio" id="raising_mode" v-model="merge_mode" v-bind:value="'raising'">
                <label class="form-check-label" for="raising_mode">Earlier images are darker, such that stars will fade in.</label>
              </div>

            </b-card-text>
          </b-card>
        </b-card-group>
      </div>



    </form>

    <h4><b-icon icon="star"></b-icon> Execution</h4>
    <b-button v-on:click="$emit('start-processing')" variant="success">Start processing</b-button>
    <br><br>

    <h6>
      Loading lightframes
      <b-icon icon="check-circle" v-if="state.loading_done" variant="success"></b-icon>
      <b-icon icon="arrow-clockwise" animation="spin" v-if="state.loading_done === false"></b-icon>
    </h6>

    <b-progress class="mt-2" :max="state.count_lights">
      <b-progress-bar :value="state.count_loaded_lights" variant="success">
        <span><strong>{{ state.count_loaded_lights }} / {{ state.count_lights }}</strong></span>
      </b-progress-bar>
      <b-progress-bar :value="state.count_loading_lights" animated show-value></b-progress-bar>
    </b-progress>
    <br>

    <h6>
      Merging images
      <b-icon icon="check-circle" v-if="state.merging_done" variant="success"></b-icon>
      <b-icon icon="arrow-clockwise" animation="spin" v-if="state.merging_done === false"></b-icon>
    </h6>

    <b-progress class="mt-2" :max="state.count_lights - 1" show-value>
      <b-progress-bar :value="state.count_merged" variant="success">
        <span><strong>{{ state.count_merged }} / {{ state.count_lights - 1 }}</strong></span>
      </b-progress-bar>
      <b-progress-bar :value="state.count_merging" animated show-value></b-progress-bar>
    </b-progress>
  </div>
</template>

<script>
import { listen } from 'tauri/api/event'
import { save } from 'tauri/api/dialog'

let vue = undefined

export default {
  name: "ManageProcessing",
  emits: {
    'start-processing': null
  },
  data: function () {
    return {
      output_path: null,
      merge_mode: "normal",
      state: {},
    }
  },
  created() { vue = this; },
  methods: {
    choose_output: function () {
      let parent = this
      save({filter: "*.dng"}).then(function (res) {
        parent.output_path = res
      })
    },
    update_state: function (updated_state) {
      this.state = updated_state.payload
    }
  },
}

listen('state_change', payload => { vue.update_state(payload) })
</script>

<style scoped>

</style>