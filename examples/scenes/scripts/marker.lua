local Marker = {}

function Marker.ready(self)
    local shade = self.node:getProperty('shade')
    self.node:findChild('Visual'):setProperty('color', {shade, 0.35, 0.85, 1})
end

return Marker
