SELECT 
    COUNT(P.Id) AS TotalPosts,
    SUM(P.ViewCount) AS TotalViewCount,
    SUM(P.Score) AS TotalScore,
    AVG(U.Reputation) AS AverageUserReputation,
    (SELECT 
        B.Name 
     FROM 
        Badges B 
     WHERE 
        B.UserId = U.Id 
     ORDER BY 
        B.Date DESC, B.Id 
     LIMIT 1) AS RecentBadge
FROM 
    Posts P
JOIN 
    Users U ON P.OwnerUserId = U.Id
GROUP BY 
    U.Id
ORDER BY 
    TotalScore DESC
LIMIT 10;