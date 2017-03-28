# Resources

## `graph_logistic_regression.pb`

This model is a Logistic Regression model `y = softmax(W * x)`, where
```
      .2   .3   .5   .7  .11
W =  .13  .17  .19  .23  .29
     .31  .37  .41  .43  .47
```

## `graph_crf.pb`

This model is a Conditional Random Field (CRF). The unary potentials are a linear mapping of the inputs with parameter `W`, and transition matrix is `psi`, where
```
      .2   .3   .5   .7  .11
W =  .13  .17  .19  .23  .29
     .31  .37  .41  .43  .47

        0.   1.   2.   3.   4.
        5.   6.   7.   8.   9.
psi =  10.  11.  12.  13.  14.
       15.  16.  17.  18.  19.
       20.  21.  22.  23.  24.
```
