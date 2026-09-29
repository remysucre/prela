
SELECT 
    U.Id AS UserId,
    U.DisplayName AS UserName,
    COUNT(P.Id) AS PostCount,
    SUM(P.ViewCount) AS TotalViews,
    SUM(P.Score) AS TotalScore,
    SUM(epoch_us(P.CreationDate))::DOUBLE / COUNT(P.CreationDate) / 1e6 AS AveragePostCreationDate
FROM 
    Users U
LEFT JOIN 
    Posts P ON U.Id = P.OwnerUserId
GROUP BY 
    U.Id, U.DisplayName
ORDER BY 
    PostCount DESC;
