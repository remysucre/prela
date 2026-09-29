SELECT 
    pt.Name AS PostType,
    COUNT(p.Id) AS TotalPosts,
    SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore,
    SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
    SUM(CASE WHEN p.CreationDate IS NOT NULL THEN date_diff('microsecond', p.CreationDate, cast('2024-10-01 12:34:56' as timestamp)) ELSE NULL END)::DOUBLE / COUNT(p.CreationDate) / 1e6 AS AvgPostAgeInSeconds
FROM 
    Posts p
JOIN 
    PostTypes pt ON p.PostTypeId = pt.Id
GROUP BY 
    pt.Name
ORDER BY 
    TotalPosts DESC;